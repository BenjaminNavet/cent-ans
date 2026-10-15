//! Minimal BC1 (DXT1) encoder for the ground texture arrays: the 2048 px "high" packs of the TX
//! ground (ADR 0236/0244) would take 16 MB per layer as RGBA8; Godot imports them as VRAM-
//! compressed BC1 (`compress/mode=2`, 0.5 byte per texel), so do the same on the loading thread.
//!
//! Real-time scheme of J.M.P. van Waveren ("Real-Time DXT Compression", 2006): bounding box of the
//! block, inset by 1/16 of its range, diagonal chosen by the sign of the covariance of red and blue
//! with green, then the nearest of the four palette colours per texel. Opaque 4-colour mode only.

/// Bytes of one BC1 level of `w x h` texels.
pub fn level_bytes(w: usize, h: usize) -> usize {
    w.max(1).div_ceil(4) * h.max(1).div_ceil(4) * 8
}

fn to565(c: [i32; 3]) -> u16 {
    let r = (c[0].clamp(0, 255) as u16 * 31 + 127) / 255;
    let g = (c[1].clamp(0, 255) as u16 * 63 + 127) / 255;
    let b = (c[2].clamp(0, 255) as u16 * 31 + 127) / 255;
    (r << 11) | (g << 5) | b
}

fn from565(v: u16) -> [i32; 3] {
    let r = ((v >> 11) & 31) as i32;
    let g = ((v >> 5) & 63) as i32;
    let b = (v & 31) as i32;
    [
        (r << 3) | (r >> 2),
        (g << 2) | (g >> 4),
        (b << 3) | (b >> 2),
    ]
}

/// Encodes one 4 x 4 block of RGB texels into 8 bytes.
pub fn encode_block(px: &[[u8; 3]; 16]) -> [u8; 8] {
    let mut lo = [255i32; 3];
    let mut hi = [0i32; 3];
    let mut mean = [0i32; 3];
    for p in px {
        for c in 0..3 {
            let v = p[c] as i32;
            lo[c] = lo[c].min(v);
            hi[c] = hi[c].max(v);
            mean[c] += v;
        }
    }
    for m in &mut mean {
        *m = (*m + 8) / 16;
    }
    // Diagonal of the box: flip red / blue when they run against green.
    for c in [0usize, 2] {
        let cov: i32 = px
            .iter()
            .map(|p| (p[c] as i32 - mean[c]) * (p[1] as i32 - mean[1]))
            .sum();
        if cov < 0 {
            std::mem::swap(&mut lo[c], &mut hi[c]);
        }
    }
    for c in 0..3 {
        let inset = (hi[c] - lo[c]) / 16;
        hi[c] -= inset;
        lo[c] += inset;
    }
    let (mut c0, mut c1) = (to565(hi), to565(lo));
    if c0 < c1 {
        std::mem::swap(&mut c0, &mut c1);
    }
    let mut out = [0u8; 8];
    out[0..2].copy_from_slice(&c0.to_le_bytes());
    out[2..4].copy_from_slice(&c1.to_le_bytes());
    if c0 == c1 {
        return out; // every index 0
    }
    let (a, b) = (from565(c0), from565(c1));
    let palette = [
        a,
        b,
        [
            (2 * a[0] + b[0]) / 3,
            (2 * a[1] + b[1]) / 3,
            (2 * a[2] + b[2]) / 3,
        ],
        [
            (a[0] + 2 * b[0]) / 3,
            (a[1] + 2 * b[1]) / 3,
            (a[2] + 2 * b[2]) / 3,
        ],
    ];
    let mut indices = 0u32;
    for (i, p) in px.iter().enumerate() {
        let mut best = (i32::MAX, 0u32);
        for (k, q) in palette.iter().enumerate() {
            let d: i32 = (0..3).map(|c| (p[c] as i32 - q[c]).pow(2)).sum();
            if d < best.0 {
                best = (d, k as u32);
            }
        }
        indices |= best.1 << (2 * i);
    }
    out[4..8].copy_from_slice(&indices.to_le_bytes());
    out
}

/// Encodes an RGBA8 level (alpha ignored) of `w x h` texels; edge blocks repeat the last texel.
pub fn encode_level(rgba: &[u8], w: usize, h: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(level_bytes(w, h));
    for by in 0..h.max(1).div_ceil(4) {
        for bx in 0..w.max(1).div_ceil(4) {
            let mut px = [[0u8; 3]; 16];
            for (i, p) in px.iter_mut().enumerate() {
                let x = (bx * 4 + i % 4).min(w - 1);
                let y = (by * 4 + i / 4).min(h - 1);
                let o = (y * w + x) * 4;
                *p = [rgba[o], rgba[o + 1], rgba[o + 2]];
            }
            out.extend_from_slice(&encode_block(&px));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_block(b: &[u8]) -> Vec<[i32; 3]> {
        let c0 = u16::from_le_bytes([b[0], b[1]]);
        let c1 = u16::from_le_bytes([b[2], b[3]]);
        let (a, z) = (from565(c0), from565(c1));
        let pal = [
            a,
            z,
            [
                (2 * a[0] + z[0]) / 3,
                (2 * a[1] + z[1]) / 3,
                (2 * a[2] + z[2]) / 3,
            ],
            [
                (a[0] + 2 * z[0]) / 3,
                (a[1] + 2 * z[1]) / 3,
                (a[2] + 2 * z[2]) / 3,
            ],
        ];
        let idx = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
        (0..16)
            .map(|i| pal[((idx >> (2 * i)) & 3) as usize])
            .collect()
    }

    #[test]
    fn flat_block_round_trips() {
        let px = [[120u8, 100, 60]; 16];
        let out = decode_block(&encode_block(&px));
        for p in out {
            assert!((p[0] - 120).abs() <= 4 && (p[1] - 100).abs() <= 2 && (p[2] - 60).abs() <= 4);
        }
    }

    #[test]
    fn gradient_block_is_close() {
        let mut px = [[0u8; 3]; 16];
        for (i, p) in px.iter_mut().enumerate() {
            let v = (i * 4) as u8;
            *p = [60 + v, 50 + v, 30 + v / 2];
        }
        let out = decode_block(&encode_block(&px));
        let err: i32 = px
            .iter()
            .zip(&out)
            .map(|(p, q)| (0..3).map(|c| (p[c] as i32 - q[c]).abs()).max().unwrap())
            .max()
            .unwrap();
        assert!(err <= 16, "max error {err}");
    }

    #[test]
    fn level_sizes() {
        assert_eq!(level_bytes(2048, 2048), 512 * 512 * 8);
        assert_eq!(level_bytes(2, 2), 8);
        assert_eq!(encode_level(&[7u8; 2 * 2 * 4], 2, 2).len(), 8);
    }
}
