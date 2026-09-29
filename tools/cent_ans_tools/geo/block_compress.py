"""GPU-ready copies of the campaign's world-wide rasters (lot OMR-R2).

Relief shade: BC5 (RGTC RG) with its full mipmap chain.

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

Wetlands (``wetlands.png``, 7168 x 6144 RGB8, 98 % black blocks): BC1 (DXT1),
half a byte per pixel (22 MB instead of 126 MB, 176 MB once padded to RGBA8 on
the GPU), ``wetlands_bc1_<i>.bin`` recorded under ``map.json.wetlands_gpu``.
"""

from __future__ import annotations

import json
import zlib
from dataclasses import dataclass
from pathlib import Path

import numpy as np

BC5_STEM = "relief_shade_bc5"
WETLANDS_STEM = "wetlands_bc1"
#: ``map.json`` keys of the GPU copies.
RELIEF_KEY = "relief_shade.bc5"
WETLANDS_KEY = "wetlands_gpu"
#: Largest raw part before zlib (parts are equal; ≈ 15-20 MB per file once compressed).
PART_RAW_BYTES = 32 * 1024 * 1024
#: Rows encoded at once (bounds the numpy temporaries on the 14336-wide raster).
ENCODE_ROWS = 512
ZLIB_LEVEL = 9


@dataclass(frozen=True)
class GpuResult:
    """Written parts and their ``map.json`` metadata."""

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


def encode_bc1(image: np.ndarray) -> bytes:
    """BC1 (DXT1, four-colour mode) bytes of an H x W x 3 uint8 image, no mipmaps.

    Endpoints are the corners of the block's bounding box (max, min) in RGB565;
    each texel takes the nearest of the four colours along that diagonal. Flat
    blocks (black sea and land without wetlands: 98 % of ``wetlands.png``) are
    exact.
    """
    height, width = image.shape[:2]
    pad_h, pad_w = (-height) % 4, (-width) % 4
    if pad_h or pad_w:
        image = np.pad(image, ((0, pad_h), (0, pad_w), (0, 0)), mode="edge")
    rows, cols = image.shape[0] // 4, image.shape[1] // 4
    chunks = []
    for top in range(0, rows, ENCODE_ROWS // 4):
        bottom = min(rows, top + ENCODE_ROWS // 4)
        part = image[top * 4 : bottom * 4]
        blocks = part.reshape(bottom - top, 4, cols, 4, 3).transpose(0, 2, 1, 3, 4)
        blocks = blocks.reshape(bottom - top, cols, 16, 3).astype(np.int32)
        hi = _rgb565(blocks.max(axis=2))
        lo = _rgb565(blocks.min(axis=2))
        e0 = _expand565(hi).astype(np.float64)
        e1 = _expand565(lo).astype(np.float64)
        axis = e1 - e0
        length2 = (axis * axis).sum(-1)
        t = ((blocks - e0[:, :, None, :]) * axis[:, :, None, :]).sum(-1)
        t = t / np.maximum(length2, 1e-9)[..., None]
        step = np.clip(np.rint(t * 3.0), 0, 3).astype(np.uint32)
        # Codes BC1 (c0 > c1) : 0 = c0, 1 = c1, 2 = ⅔ c0 + ⅓ c1, 3 = ⅓ c0 + ⅔ c1.
        codes = np.choose(step, [0, 2, 3, 1]).astype(np.uint32)
        flat = hi == lo
        codes[flat] = 0
        shifts = (np.arange(16, dtype=np.uint32) * 2)[None, None, :]
        bits = np.bitwise_or.reduce(codes << shifts, axis=-1)
        out = np.empty((bottom - top, cols, 8), dtype=np.uint8)
        out[..., 0] = hi & 0xFF
        out[..., 1] = hi >> 8
        out[..., 2] = lo & 0xFF
        out[..., 3] = lo >> 8
        for k in range(4):
            out[..., 4 + k] = (bits >> (8 * k)) & 0xFF
        chunks.append(out.tobytes())
    return b"".join(chunks)


def _rgb565(rgb: np.ndarray) -> np.ndarray:
    r = np.rint(rgb[..., 0] * 31 / 255).astype(np.uint32)
    g = np.rint(rgb[..., 1] * 63 / 255).astype(np.uint32)
    b = np.rint(rgb[..., 2] * 31 / 255).astype(np.uint32)
    return (r << 11) | (g << 5) | b


def _expand565(color: np.ndarray) -> np.ndarray:
    r = (color >> 11) & 31
    g = (color >> 5) & 63
    b = color & 31
    return np.stack(
        [(r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2)], axis=-1
    )


def decode_bc1(data: bytes, height: int, width: int) -> np.ndarray:
    """Inverse of :func:`encode_bc1` (four-colour mode and flat blocks), for tests."""
    rows, cols = (height + 3) // 4, (width + 3) // 4
    blocks = np.frombuffer(data, dtype=np.uint8).reshape(rows, cols, 8)
    c0 = blocks[..., 0].astype(np.uint32) | (blocks[..., 1].astype(np.uint32) << 8)
    c1 = blocks[..., 2].astype(np.uint32) | (blocks[..., 3].astype(np.uint32) << 8)
    e0 = _expand565(c0).astype(np.float64)
    e1 = _expand565(c1).astype(np.float64)
    bits = np.zeros((rows, cols), dtype=np.uint32)
    for k in range(4):
        bits |= blocks[..., 4 + k].astype(np.uint32) << (8 * k)
    codes = np.stack([(bits >> (2 * i)) & 3 for i in range(16)], axis=-1)
    weight = np.choose(codes, [0.0, 3.0, 1.0, 2.0]) / 3.0
    colors = (
        e0[:, :, None, :] * (1 - weight[..., None])
        + e1[:, :, None, :] * weight[..., None]
    )
    image = colors.reshape(rows, cols, 4, 4, 3).transpose(0, 2, 1, 3, 4)
    return image.reshape(rows * 4, cols * 4, 3)[:height, :width]


def write_parts(data: bytes, map_dir: Path, stem: str) -> tuple[list[Path], list[int]]:
    """Write ``data`` as equal zlib parts ``<stem>_<i>.bin``; returns paths and raw sizes."""
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
    return paths, sizes


def _meta(
    fmt: str,
    width: int,
    height: int,
    mipmaps: bool,
    data: bytes,
    stem: str,
    sizes: list[int],
) -> dict:
    return {
        "format": fmt,
        "size_px": [width, height],
        "mipmaps": mipmaps,
        "bytes": len(data),
        "compression": "deflate",
        "pattern": stem + "_{part}.bin",
        "part_bytes": sizes,
    }


def write_relief_bc5(
    image: np.ndarray, map_dir: Path, stem: str = BC5_STEM
) -> GpuResult:
    """BC5 mip chain of the shade raster (H x W x 2) in parts; ``relief_shade.bc5`` metadata."""
    height, width = image.shape[:2]
    data = encode_chain(image)
    expected = chain_bytes(width, height)
    if len(data) != expected:
        raise ValueError(f"BC5 chain is {len(data)} bytes, expected {expected}")
    paths, sizes = write_parts(data, map_dir, stem)
    return GpuResult(
        paths=paths, meta=_meta("rgtc_rg", width, height, True, data, stem, sizes)
    )


def write_wetlands_bc1(
    image: np.ndarray, map_dir: Path, stem: str = WETLANDS_STEM
) -> GpuResult:
    """BC1 copy of ``wetlands.png`` (H x W x 3, no mipmaps); ``wetlands_gpu`` metadata."""
    height, width = image.shape[:2]
    data = encode_bc1(image)
    paths, sizes = write_parts(data, map_dir, stem)
    return GpuResult(
        paths=paths, meta=_meta("dxt1", width, height, False, data, stem, sizes)
    )


def update_map_json(map_dir: Path, key: str, meta: dict) -> None:
    """Record ``meta`` under ``key`` of ``map.json`` (``relief_shade.bc5``, ``wetlands_gpu``)."""
    path = map_dir / "map.json"
    metadata = json.loads(path.read_text(encoding="utf-8"))
    node = metadata
    *parents, leaf = key.split(".")
    for name in parents:
        node = node.setdefault(name, {})
    node[leaf] = meta
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


def build_from_pngs(map_dir: Path) -> list[Path]:
    """``geo gpu-textures``: BC5 relief and BC1 wetlands from the PNGs in ``map_dir``."""
    from PIL import Image

    written: list[Path] = []
    relief = write_relief_bc5(read_png_bands(map_dir), map_dir)
    update_map_json(map_dir, RELIEF_KEY, relief.meta)
    written += relief.paths
    wet_png = map_dir / "wetlands.png"
    if wet_png.exists():
        with Image.open(wet_png) as image:
            wet = write_wetlands_bc1(np.asarray(image.convert("RGB")), map_dir)
        update_map_json(map_dir, WETLANDS_KEY, wet.meta)
        written += wet.paths
    return written
