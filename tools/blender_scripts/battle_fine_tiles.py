"""Lot FG3: shared tileable detail maps of the fine figures, and small PNG helpers.

Pure numpy (runs inside Blender or with ``uv run --project tools python``). Each tile is a
periodic height field turned into a tangent-space normal (OpenGL convention: green = up of
the image), plus a relief channel (albedo modulation, mean 0.5) and a roughness channel:

====  =========  ========  =========================================================
layer name       size (m)  content
====  =========  ========  =========================================================
0     mail       0.072     riveted rings, 9 mm pitch, 12 interlinked rows (4-in-1)
1     weave      0.024     plain weave (tabby) of 1 mm woollen threads, uneven yarn
2     felt       0.08      felted / padded wool (gambeson), lumps and fibres
3     leather    0.12      grain creases and scuffs
4     hammered   0.16      shallow hammer dents of a forged plate, polished patches
5     wood       0.3       grain along the shaft, knots
6     skin       0.06      pores and fine wrinkles
7     hair       0.05      strands running down
====  =========  ========  =========================================================

The sizes are those of ``FG3_TILE_SIZE`` in ``battle_soldier_skinned.gdshader``; the shader
maps them onto the rest-pose position of the figure (metres).

Output: ``fine_detail.png`` (vertical strip, one 512² layer per slice), imported by Godot as
a ``Texture2DArray`` (VRAM, BC7).
"""

import struct
import zlib

import numpy as np

TILE = 512
LAYERS = ("mail", "weave", "felt", "leather", "hammered", "wood", "skin", "hair")
TILE_SIZE_M = (0.072, 0.024, 0.08, 0.12, 0.16, 0.3, 0.06, 0.05)


# --- PNG ------------------------------------------------------------------------------


def write_png(path, rgba):
    """Write an ``(h, w, 4)`` uint8 array (row 0 = top) as an RGBA PNG (Sub filter)."""
    rgba = np.ascontiguousarray(rgba, dtype=np.uint8)
    h, w, _c = rgba.shape
    sub = rgba.astype(np.int16)
    sub[:, 1:] -= rgba[:, :-1].astype(np.int16)
    rows = (sub & 0xFF).astype(np.uint8).reshape(h, w * 4)
    raw = np.concatenate([np.ones((h, 1), np.uint8), rows], axis=1).tobytes()

    def chunk(tag, data):
        crc = zlib.crc32(tag + data) & 0xFFFFFFFF
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", crc)

    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n")
        f.write(chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)))
        f.write(chunk(b"IDAT", zlib.compress(raw, 9)))
        f.write(chunk(b"IEND", b""))


def read_png(path):
    """Read an RGBA PNG written by `write_png` (Sub or None filters); row 0 = top."""
    with open(path, "rb") as f:
        data = f.read()
    pos = 8
    w = h = 0
    idat = b""
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos : pos + 4])
        tag = data[pos + 4 : pos + 8]
        body = data[pos + 8 : pos + 8 + length]
        if tag == b"IHDR":
            w, h = struct.unpack(">II", body[:8])
            if body[8:10] != b"\x08\x06":
                raise ValueError(f"{path}: not RGBA8")
        elif tag == b"IDAT":
            idat += body
        pos += 12 + length
    raw = np.frombuffer(zlib.decompress(idat), np.uint8).reshape(h, w * 4 + 1)
    filters = raw[:, 0]
    px = raw[:, 1:].reshape(h, w, 4).astype(np.uint32)
    if np.any((filters != 0) & (filters != 1)):
        raise ValueError(f"{path}: unsupported PNG filter")
    rows = filters == 1
    px[rows] = np.cumsum(px[rows], axis=1) & 0xFF
    return px.astype(np.uint8)


# --- Periodic fields ------------------------------------------------------------------


def periodic_noise(n, freq, rng, aniso=(1.0, 1.0), octaves=1, gain=0.5):
    """Tileable band-limited noise in [0, 1]; `freq` cycles per tile, `aniso` = (u, v) scale."""
    fy = np.fft.fftfreq(n)[:, None] * n
    fx = np.fft.fftfreq(n)[None, :] * n
    out = np.zeros((n, n))
    amp = 1.0
    for o in range(octaves):
        f0 = freq * (2**o)
        r = np.sqrt((fx / aniso[0]) ** 2 + (fy / aniso[1]) ** 2)
        spectrum = np.exp(-0.5 * ((r - f0) / (0.5 * f0 + 0.5)) ** 2)
        spectrum[0, 0] = 0.0
        white = rng.standard_normal((n, n)) + 1j * rng.standard_normal((n, n))
        field = np.real(np.fft.ifft2(white * spectrum))
        out += amp * field / (field.std() + 1e-9)
        amp *= gain
    lo, hi = np.percentile(out, (0.5, 99.5))
    return np.clip((out - lo) / (hi - lo + 1e-9), 0.0, 1.0)


def grid(n):
    """Pixel-centre coordinates (u right, w down) in [0, 1)."""
    c = (np.arange(n) + 0.5) / n
    return np.meshgrid(c, c)  # (u, w): arrays [row, col]


def wrap_delta(a):
    """Signed shortest difference on the unit torus."""
    return a - np.round(a)


def normal_from_height(h, depth_px):
    """Tangent-space normal (x right, y up of the image) of a periodic height field.

    `depth_px`: relief height of 1.0 in pixels (steepness).
    """
    dx = (np.roll(h, -1, axis=1) - np.roll(h, 1, axis=1)) * 0.5 * depth_px
    dup = (np.roll(h, 1, axis=0) - np.roll(h, -1, axis=0)) * 0.5 * depth_px
    n = np.stack([-dx, -dup, np.ones_like(h)], axis=-1)
    return n / np.linalg.norm(n, axis=-1, keepdims=True)


def pack(normal, relief, rough):
    """RGBA uint8: RG normal, B relief (0.5 = neutral), A roughness factor."""
    out = np.zeros(normal.shape[:2] + (4,), np.float64)
    out[..., 0] = normal[..., 0] * 0.5 + 0.5
    out[..., 1] = normal[..., 1] * 0.5 + 0.5
    out[..., 2] = relief
    out[..., 3] = rough
    return np.clip(np.round(out * 255.0), 0, 255).astype(np.uint8)


def recentre(x, mean=0.5, spread=None):
    """Shift (and optionally scale) `x` so that its mean is `mean`."""
    x = x - x.mean()
    if spread is not None:
        x = x / (np.abs(x).max() + 1e-9) * spread
    return np.clip(x + mean, 0.0, 1.0)


# --- Tiles ----------------------------------------------------------------------------


def tile_mail(n, rng):
    """Riveted mail: 8 rings across, 12 rows (interlinked 4-in-1), rows leaning alternately."""
    cols, rows = 8, 12
    u, w = grid(n)
    pu, pw = 1.0 / cols, 1.0 / rows
    r0 = 0.44 * pu  # ring radius (to the wire's centre): neighbours overlap
    wire = 0.10 * pu
    height = np.zeros((n, n))
    cover = np.zeros((n, n))
    for row in range(rows):
        cy = (row + 0.5) * pw
        shift = 0.5 * pu * (row % 2)
        lean = 1.0 if row % 2 == 0 else -1.0
        for col in range(cols):
            cx = (col + 0.5) * pu + shift
            dx = wrap_delta(u - cx) / pu
            dy = wrap_delta(w - cy) / pu
            # Rings seen at an angle (oval), the upper arc of a row over the row below.
            d = np.sqrt(dx**2 + (dy * 1.35) ** 2) * pu
            t = np.clip(1.0 - ((d - r0) / wire) ** 2, 0.0, 1.0)
            ring = np.sqrt(t) * (0.8 + 0.2 * lean * np.clip(dx * 2.0, -1, 1))
            ring *= 0.8 + 0.2 * np.clip(-dy * 2.5, -1, 1)
            rv = np.exp(-(((dx - 0.44) ** 2 + (dy * 1.35) ** 2) / 0.003))
            ring = np.maximum(ring, 1.1 * rv * (t > 0))
            height = np.maximum(height, ring)
            cover = np.maximum(cover, t)
    wobble = periodic_noise(n, 20, rng) * 0.12
    height = np.clip(height * (0.92 + wobble), 0.0, 1.2)
    normal = normal_from_height(height / 1.2, depth_px=9.0)
    relief = 0.08 + 0.92 * np.clip(height / 1.1, 0.0, 1.0) ** 0.7
    relief *= 0.85 + 0.3 * periodic_noise(n, 6, rng)
    rough = 0.15 + 0.75 * np.clip(cover, 0.0, 1.0)
    return pack(normal, np.clip(relief, 0.0, 1.0), rough)


def tile_weave(n, rng):
    """Plain weave: 24 warp and 24 weft threads crossing over and under."""
    threads = 24
    u, w = grid(n)
    pu = u * threads
    pw = w * threads
    iu = np.floor(pu).astype(int)
    iw = np.floor(pw).astype(int)
    fu = pu - iu
    fw = pw - iw
    over = (iu + iw) % 2 == 0  # warp over weft
    # Thread cross-sections (round), thickness varying along the yarn (slubs).
    slub_u = periodic_noise(n, 12, rng, aniso=(0.25, 1.0))
    slub_w = periodic_noise(n, 12, rng, aniso=(1.0, 0.25))
    warp = np.sqrt(np.clip(1.0 - ((fu - 0.5) / (0.42 + 0.08 * slub_u)) ** 2, 0, 1))
    weft = np.sqrt(np.clip(1.0 - ((fw - 0.5) / (0.42 + 0.08 * slub_w)) ** 2, 0, 1))
    # The thread on top arches over the crossing.
    arch_w = np.sin(np.pi * fw)
    arch_u = np.sin(np.pi * fu)
    height = np.where(
        over, 0.55 * warp + 0.45 * arch_w * warp, 0.55 * weft + 0.45 * arch_u * weft
    )
    height += 0.08 * periodic_noise(n, 60, rng)
    normal = normal_from_height(height, depth_px=3.0)
    relief = recentre(height * 0.8 + 0.2 * slub_u * slub_w, spread=0.35)
    rough = 0.55 + 0.25 * periodic_noise(n, 30, rng)
    return pack(normal, relief, rough)


def tile_felt(n, rng):
    """Felted, padded wool: soft lumps, pills and short fibres."""
    lumps = periodic_noise(n, 5, rng, octaves=3, gain=0.45)
    fibres = periodic_noise(n, 70, rng, aniso=(1.0, 0.35))
    pills = periodic_noise(n, 40, rng) ** 6
    height = 0.7 * lumps + 0.18 * fibres + 0.3 * pills
    normal = normal_from_height(height, depth_px=6.0)
    relief = recentre(0.6 * lumps + 0.4 * fibres, spread=0.3)
    rough = 0.75 + 0.2 * fibres
    return pack(normal, relief, np.clip(rough, 0, 1))


def tile_leather(n, rng):
    """Leather: cellular grain creases, broad wrinkles and scuffs."""
    cells = 34
    u, w = grid(n)
    pts = rng.random((cells * cells // 6, 2))
    d1 = np.full((n, n), 9.0)
    d2 = np.full((n, n), 9.0)
    for p in pts:
        d = np.sqrt(wrap_delta(u - p[0]) ** 2 + wrap_delta(w - p[1]) ** 2)
        d2 = np.where(d < d1, d1, np.minimum(d2, d))
        d1 = np.minimum(d1, d)
    crease = np.clip((d2 - d1) * cells * 1.4, 0.0, 1.0)
    wrinkles = periodic_noise(n, 4, rng, aniso=(1.0, 0.3), octaves=2)
    scuffs = periodic_noise(n, 18, rng, aniso=(0.2, 1.0)) ** 5
    height = 0.55 * crease + 0.35 * wrinkles - 0.25 * scuffs
    normal = normal_from_height(height, depth_px=5.0)
    relief = recentre(0.5 * crease + 0.3 * wrinkles + 0.4 * scuffs, spread=0.35)
    rough = 0.5 + 0.3 * crease - 0.2 * scuffs
    return pack(normal, relief, np.clip(rough, 0, 1))


def tile_hammered(n, rng):
    """Forged plate: overlapping shallow hammer dents, polished and dull patches."""
    u, w = grid(n)
    height = np.zeros((n, n))
    for _ in range(170):
        cx, cy = rng.random(2)
        r = rng.uniform(0.035, 0.08)
        d2 = (wrap_delta(u - cx) ** 2 + wrap_delta(w - cy) ** 2) / (r * r)
        dent = np.clip(1.0 - d2, 0.0, 1.0) ** 1.5
        height = np.minimum(height, -dent * rng.uniform(0.6, 1.0))
    height += 0.25 * periodic_noise(n, 3, rng, octaves=2)
    normal = normal_from_height(height, depth_px=5.0)
    relief = recentre(height, spread=0.12)
    polish = periodic_noise(n, 3, rng, octaves=2)
    scratches = periodic_noise(n, 60, rng, aniso=(0.08, 1.0)) ** 4
    rough = 0.35 + 0.5 * polish + 0.25 * scratches
    return pack(normal, relief, np.clip(rough, 0, 1))


def tile_wood(n, rng):
    """Wood grain running along the image (shafts, hafts, bows), a knot or two."""
    u, w = grid(n)
    warp = periodic_noise(n, 2, rng, octaves=2) * 0.08
    rings = np.sin((u + warp) * 2 * np.pi * 14.0) * 0.5 + 0.5
    fibres = periodic_noise(n, 50, rng, aniso=(1.0, 0.06))
    height = 0.5 * rings**3 + 0.4 * fibres
    normal = normal_from_height(height, depth_px=3.0)
    relief = recentre(0.6 * (1.0 - rings**3) + 0.4 * fibres, spread=0.4)
    rough = 0.55 + 0.3 * fibres
    return pack(normal, relief, np.clip(rough, 0, 1))


def tile_skin(n, rng):
    """Skin: pores and fine criss-cross wrinkles."""
    pores = periodic_noise(n, 90, rng) ** 5
    lines_a = periodic_noise(n, 25, rng, aniso=(0.15, 1.0))
    lines_b = periodic_noise(n, 25, rng, aniso=(1.0, 0.15))
    height = 0.5 * lines_a * lines_b - 0.5 * pores
    normal = normal_from_height(height, depth_px=2.0)
    relief = recentre(-pores + 0.3 * lines_a, spread=0.12)
    rough = 0.55 + 0.25 * pores
    return pack(normal, relief, np.clip(rough, 0, 1))


def tile_hair(n, rng):
    """Hair strands running down the image (hair, beards, manes), clumped."""
    strands = periodic_noise(n, 48, rng, aniso=(1.0, 0.04), octaves=2)
    clumps = periodic_noise(n, 8, rng, aniso=(1.0, 0.1))
    height = 0.6 * strands + 0.4 * clumps
    normal = normal_from_height(height, depth_px=5.0)
    relief = recentre(0.7 * strands + 0.3 * clumps, spread=0.45)
    rough = 0.45 + 0.4 * (1.0 - strands)
    return pack(normal, relief, np.clip(rough, 0, 1))


BUILDERS = {
    "mail": tile_mail,
    "weave": tile_weave,
    "felt": tile_felt,
    "leather": tile_leather,
    "hammered": tile_hammered,
    "wood": tile_wood,
    "skin": tile_skin,
    "hair": tile_hair,
}


def make_tiles(path, n=TILE, seed=1337):
    """Build every layer (deterministic) and write the vertical strip to `path`."""
    layers = []
    for i, name in enumerate(LAYERS):
        rng = np.random.default_rng(seed + i)
        layers.append(BUILDERS[name](n, rng))
        print(f"TILE {i} {name} {TILE_SIZE_M[i]} m")
    strip = np.concatenate(layers, axis=0)
    write_png(path, strip)
    return strip


def strip_layer(strip, index, size):
    """Slice `index` (row 0 = top) of a vertical strip of `size`² layers."""
    return strip[index * size : (index + 1) * size]


def dilate(rgba, covered, steps):
    """Push covered pixels into uncovered ones for `steps` pixels (mipmap-safe margins)."""
    img = rgba.astype(np.float32)
    cov = covered.astype(bool).copy()
    for _ in range(steps):
        if cov.all():
            break
        acc = np.zeros_like(img)
        cnt = np.zeros(cov.shape, np.float32)
        for dy, dx in (
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ):
            m = np.roll(np.roll(cov, dy, 0), dx, 1)
            v = np.roll(np.roll(img, dy, 0), dx, 1)
            acc += v * m[..., None]
            cnt += m
        grow = (~cov) & (cnt > 0)
        img[grow] = acc[grow] / cnt[grow][:, None]
        cov |= grow
    return img, cov


def horse_pack(colour, normal, ao, size=1024):
    """Horse coat map (float arrays in [0, 1], row 0 = top) reduced to `size`².

    RG = normal (tangent space of the source), B = high-pass luminance of the coat colour
    (0.5 = neutral: the shader keeps the robe tint), A = ambient occlusion normalised.
    """

    def resize(img):
        h, w = img.shape[:2]
        fy, fx = h // size, w // size
        if fy >= 1 and fx >= 1 and h % size == 0 and w % size == 0:
            return img.reshape(size, fy, size, fx, *img.shape[2:]).mean(axis=(1, 3))
        ys = (np.arange(size) * h // size).astype(int)
        xs = (np.arange(size) * w // size).astype(int)
        return img[ys][:, xs]

    col = resize(colour[..., :3])
    nrm = resize(normal[..., :3])
    occ = resize(ao[..., 0] if ao.ndim == 3 else ao)
    lum = col @ np.array([0.3, 0.59, 0.11])
    # Low-pass by an FFT gaussian (periodic: the UV islands do not wrap, the edges are
    # padding anyway).
    f = np.fft.fft2(lum)
    fy = np.fft.fftfreq(size)[:, None]
    fx = np.fft.fftfreq(size)[None, :]
    blur = np.real(np.fft.ifft2(f * np.exp(-((fx**2 + fy**2) * (size / 24.0) ** 2))))
    high = np.clip(0.5 + 0.9 * (lum / np.maximum(blur, 1e-3) - 1.0), 0.0, 1.0)
    occ = np.clip(occ / max(np.percentile(occ, 90), 1e-3), 0.0, 1.0)
    out = np.zeros((size, size, 4))
    n = nrm * 2.0 - 1.0
    n /= np.maximum(np.linalg.norm(n, axis=-1, keepdims=True), 1e-6)
    out[..., 0] = n[..., 0] * 0.5 + 0.5
    out[..., 1] = n[..., 1] * 0.5 + 0.5
    out[..., 2] = high
    out[..., 3] = 0.35 + 0.65 * occ
    return np.clip(np.round(out * 255.0), 0, 255).astype(np.uint8)


if __name__ == "__main__":
    import sys

    make_tiles(sys.argv[1] if len(sys.argv) > 1 else "fine_detail.png")
