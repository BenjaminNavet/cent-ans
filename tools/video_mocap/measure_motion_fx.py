# /// script
# requires-python = ">=3.10,<3.13"
# dependencies = ["opencv-python-headless>=4.9", "numpy>=1.26"]
# ///
"""Lot AS8d: measure engine, fire, grass and flag motion in free videos (ADR 0189).

Videos stay outside the repository (``~/dev/cent-ans-mocap-src/video/free/fx/``); only the
measured numbers (``data/fx/*.json``, field ``source``) and the baked flame atlas are committed.
Every subcommand prints a JSON object on stdout (and writes it with ``--out``)::

    uv run tools/video_mocap/measure_motion_fx.py trebuchet VIDEO --pivot X,Y --start-deg D
    uv run tools/video_mocap/measure_motion_fx.py winch VIDEO --pivot X,Y --start-deg D
    uv run tools/video_mocap/measure_motion_fx.py cannon VIDEO
    uv run tools/video_mocap/measure_motion_fx.py grass VIDEO
    uv run tools/video_mocap/measure_motion_fx.py flag VIDEO
    uv run tools/video_mocap/measure_motion_fx.py flame VIDEO --crop X,Y,W,H --start S --seconds T \
        --atlas OUT.png --columns 8 --rows 8 --frame-px 128

Method per subcommand:

* ``trebuchet`` / ``winch``: the arm direction is tracked frame by frame from a fixed pivot. For
  each frame the ray (within a window around the previous angle) with the longest unbroken run of
  wood-coloured pixels is the arm. Output: angle(t) resampled on a normalised time axis.
* ``cannon``: recoil from the centroid of dense motion before the smoke; flash and smoke durations
  from bright-then-grey area curves.
* ``grass`` / ``flag``: dense Farneback optical flow, mean flow signal over the frame, spectrum by
  FFT. ``flag`` also reports the spatial wavelength (distance between flow-phase wraps along the
  flag).
* ``flame``: flame pixels keyed by colour (red dominant, saturated, bright) inside a crop, loop
  closed by cross-fade, packed into an atlas (RGB = heat, A = coverage, same coding as FA2 flipbooks),
  flicker frequency from the luminance spectrum.
"""

import argparse
import json
import sys

import cv2
import numpy as np


def read_frames(path, start=0.0, seconds=None, step=1, max_width=None):
    cap = cv2.VideoCapture(str(path))
    fps = cap.get(cv2.CAP_PROP_FPS) or 25.0
    if start:
        cap.set(cv2.CAP_PROP_POS_MSEC, start * 1000.0)
    frames = []
    count = 0
    limit = None if seconds is None else int(seconds * fps)
    while limit is None or count < limit:
        ok, frame = cap.read()
        if not ok:
            break
        if count % step == 0:
            if max_width and frame.shape[1] > max_width:
                scale = max_width / frame.shape[1]
                frame = cv2.resize(
                    frame, None, fx=scale, fy=scale, interpolation=cv2.INTER_AREA
                )
            frames.append(frame)
        count += 1
    return frames, fps / step


def wood_mask(bgr):
    """Pale warm wood (red above blue, red not below green) against sky, grass and trees."""
    b, g, r = (bgr[..., i].astype(np.int16) for i in range(3))
    return ((r - b >= 6) & (r >= g - 4) & (r >= 120)).astype(np.uint8)


def arm_angle(mask, pivot, lo_deg, hi_deg, r_min=25, r_max=140):
    """Angle (degrees, image x right, y up) of the ray from the pivot with the most wood pixels."""
    h, w = mask.shape
    radii = np.arange(r_min, r_max)
    best, best_score = lo_deg, -1.0
    for angle in np.arange(lo_deg, hi_deg + 0.01, 0.5):
        xs = np.round(pivot[0] + np.cos(np.radians(angle)) * radii).astype(int)
        ys = np.round(pivot[1] - np.sin(np.radians(angle)) * radii).astype(int)
        inside = (xs >= 0) & (xs < w) & (ys >= 0) & (ys < h)
        if inside.sum() < len(radii) * 0.8:
            continue
        score = float(mask[ys[inside], xs[inside]].mean())
        if score > best_score:
            best, best_score = angle, score
    return best, best_score


def track_arm(frames, pivot, lo_deg, hi_deg):
    """Arm angle per frame: wood-coloured ray search between `lo_deg` and `hi_deg`, which keeps
    the A-frame legs (pointing down) out of the search range.
    """
    angles, scores = [], []
    for frame in frames:
        mask = cv2.morphologyEx(
            wood_mask(frame), cv2.MORPH_CLOSE, np.ones((3, 3), np.uint8)
        )
        angle, score = arm_angle(mask, pivot, lo_deg, hi_deg)
        angles.append(angle)
        scores.append(score)
    return np.array(angles), np.array(scores)


def parse_pair(text):
    return tuple(float(v) for v in text.split(","))


def cmd_trebuchet(args):
    frames, fps = read_frames(args.video, args.start, args.seconds, step=args.step)
    pivot = parse_pair(args.pivot)
    angles, runs = track_arm(frames, pivot, args.lo_deg, args.hi_deg)
    # Smooth lightly (3-tap) and unwrap.
    angles = np.convolve(
        np.pad(angles, 1, mode="edge"), [0.25, 0.5, 0.25], mode="valid"
    )
    for item in filter(None, args.override.split(",")):
        index, value = item.split(":")
        angles[int(index)] = float(value)
    result = {
        "fps": fps,
        "frames": len(angles),
        "angle_deg": [round(float(a), 2) for a in angles],
        "score": [round(float(r), 2) for r in runs],
    }
    if args.debug:
        for i in range(0, len(frames), max(1, len(frames) // 24)):
            img = frames[i].copy()
            a = np.radians(angles[i])
            tip = (int(pivot[0] + np.cos(a) * 150), int(pivot[1] - np.sin(a) * 150))
            cv2.line(img, (int(pivot[0]), int(pivot[1])), tip, (0, 0, 255), 2)
            cv2.imwrite(f"{args.debug}/arm_{i:04d}.png", img)
    return result


def cmd_swing_lut(args):
    """Normalise a tracked throw into progress(u): u = 0..1 over the swing (first movement to
    the end of the overswing), progress = (angle0 - angle) / (angle0 - angle_end) in 0..1.
    """
    with open(args.tracked) as handle:
        tracked = json.load(handle)
    angles = np.array(tracked["angle_deg"][args.first : args.last + 1], dtype=float)
    fps = tracked["fps"]
    angles = np.convolve(
        np.pad(angles, 1, mode="edge"), [0.25, 0.5, 0.25], mode="valid"
    )
    start = angles[0]
    end = angles.min()
    progress = (start - angles) / (start - end)
    # Swing starts when progress first exceeds 2 %.
    first = int(np.argmax(progress > 0.02))
    progress = progress[max(first - 1, 0) :]
    duration = (len(progress) - 1) / fps
    grid = np.linspace(0.0, 1.0, args.points)
    lut = np.interp(grid, np.linspace(0.0, 1.0, len(progress)), progress)
    lut[0], lut[-1] = 0.0, 1.0
    vertical = (start - 90.0) / (start - end)
    u_vertical = float(
        np.interp(
            vertical,
            np.maximum.accumulate(progress),
            np.linspace(0.0, 1.0, len(progress)),
        )
    )
    return {
        "swing_s": round(float(duration), 3),
        "lut": [round(float(v), 4) for v in lut],
        "angle_start_deg": round(float(start), 1),
        "angle_end_deg": round(float(end), 1),
        "arc_deg": round(float(start - end), 1),
        "vertical_phase": round(u_vertical, 3),
        "max_speed_deg_s": round(float(np.abs(np.diff(angles)).max() * fps), 1),
    }


def cmd_cannon(args):
    """Flash, smoke and recoil of a wheeled gun filmed from the side with a static camera.

    The shot frame is the first with a warm bright blob (flash). Flash duration: frames with that
    blob above 25 % of its peak. Smoke: fraction of pixels that differ from the pre-shot frame by
    more than 25 grey levels after the flash; its rise to the peak and its decay to 20 % of the
    peak. Recoil: cumulative horizontal shift of the lower half of the frame (the gun on its
    wheels) by phase correlation, in barrel lengths (``--barrel-px``).
    """
    frames, fps = read_frames(args.video, 0.0, None, max_width=480)
    gray = [cv2.cvtColor(f, cv2.COLOR_BGR2GRAY).astype(np.float32) for f in frames]
    warm = []
    for f in frames:
        b, g, r = (f[..., i].astype(np.int16) for i in range(3))
        warm.append(int(np.sum((r > 235) & (g > 165) & (b > 90) & (r - b > 40))))
    warm = np.array(warm)
    shot = int(np.argmax(warm > max(40, warm.max() * 0.3)))
    base = gray[max(shot - 3, 0)]
    changed = np.array([float(np.mean(np.abs(g - base) > 25)) for g in gray])
    peak_frame = int(np.argmax(warm))
    first, last = peak_frame, peak_frame
    while first > 0 and warm[first - 1] >= 0.1 * warm.max():
        first -= 1
    while last < len(warm) - 1 and warm[last + 1] >= 0.1 * warm.max():
        last += 1
    flash_frames = last - first + 1
    after = changed[shot:]
    peak = int(np.argmax(after))
    below = np.where(after[peak:] < 0.2 * after[peak])[0]
    decay = int(below[0]) if below.size else len(after) - peak
    h = gray[0].shape[0]
    shifts = [0.0]
    for i in range(
        max(shot - 2, 1), min(shot + int(args.recoil_seconds * fps), len(gray))
    ):
        window = cv2.createHanningWindow((gray[i].shape[1], h - h // 2), cv2.CV_32F)
        (dx, _), _ = cv2.phaseCorrelate(
            gray[i - 1][h // 2 :], gray[i][h // 2 :], window
        )
        shifts.append(shifts[-1] + dx)
    shifts = np.array(shifts)
    width = gray[0].shape[1]
    barrel_px = args.barrel_px * width / 640.0
    return {
        "fps": fps,
        "shot_frame": shot,
        "flash_s": round(flash_frames / fps, 3),
        "flash_peak_px": int(warm.max()),
        "smoke_peak_after_s": round(peak / fps, 3),
        "smoke_peak_fraction": round(float(after[peak]), 3),
        "smoke_decay_to_20pct_s": round((peak + decay) / fps, 3) if decay else None,
        "recoil_shift_px_per_frame": [round(float(v), 2) for v in np.diff(shifts)],
        "recoil_total_px": round(float(shifts[-1] - shifts[0]), 2),
        "recoil_max_px": round(float(np.abs(shifts).max()), 2),
        "recoil_barrel_lengths": round(float(np.abs(shifts).max() / barrel_px), 3),
    }


def flow_series(frames):
    prev = cv2.cvtColor(frames[0], cv2.COLOR_BGR2GRAY)
    series, fields = [], []
    for frame in frames[1:]:
        gray = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)
        flow = cv2.calcOpticalFlowFarneback(prev, gray, None, 0.5, 3, 21, 3, 5, 1.1, 0)
        series.append(flow.reshape(-1, 2).mean(axis=0))
        fields.append(flow)
        prev = gray
    return np.array(series), fields


def spectrum(signal, fps, f_min=0.1, f_max=None):
    signal = np.asarray(signal, dtype=float)
    signal = signal - signal.mean()
    window = np.hanning(len(signal))
    power = np.abs(np.fft.rfft(signal * window)) ** 2
    freqs = np.fft.rfftfreq(len(signal), 1.0 / fps)
    f_max = f_max or fps / 2
    keep = (freqs >= f_min) & (freqs <= f_max)
    return freqs[keep], power[keep]


def cmd_grass(args):
    """Wind sway of grass: per-frame median flow (robust to blade-level noise and compression
    flicker), spectrum between 0.15 and 4 Hz, and the gust envelope (1 s moving average of the
    absolute median flow).
    """
    frames, fps = read_frames(
        args.video, args.start, args.seconds, step=args.step, max_width=480
    )
    prev = cv2.cvtColor(frames[0], cv2.COLOR_BGR2GRAY)
    medians = []
    for frame in frames[1:]:
        gray = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)
        flow = cv2.calcOpticalFlowFarneback(prev, gray, None, 0.5, 3, 21, 3, 5, 1.1, 0)
        medians.append(np.median(flow.reshape(-1, 2), axis=0))
        prev = gray
    medians = np.array(medians)
    axis = np.linalg.svd(medians - medians.mean(0), full_matrices=False)[2][0]
    signal = medians @ axis
    mad = np.median(np.abs(signal - np.median(signal))) + 1e-9
    signal = np.clip(
        signal, -6 * 1.4826 * mad, 6 * 1.4826 * mad
    )  # drop loop-seam spikes
    # Integrate to displacement (sway), remove drift.
    displacement = np.cumsum(signal)
    displacement -= np.polyval(
        np.polyfit(np.arange(len(displacement)), displacement, 1),
        np.arange(len(displacement)),
    )
    freqs, power = spectrum(displacement, fps, f_min=0.15, f_max=4.0)
    order = np.argsort(-power)[:3]
    centroid = float(np.sum(freqs * power) / np.sum(power))
    envelope = np.convolve(
        np.abs(signal), np.ones(max(1, int(fps))) / max(1, int(fps)), mode="same"
    )
    return {
        "fps": fps,
        "seconds": round(len(frames) / fps, 2),
        "dominant_hz": [round(float(freqs[i]), 3) for i in order],
        "power_share": [round(float(power[i] / power.sum()), 3) for i in order],
        "spectral_centroid_hz": round(centroid, 3),
        "sway_rms_px": round(float(displacement.std()), 3),
        "speed_rms_px_per_s": round(float(signal.std() * fps), 3),
        "gust_peak_over_mean": round(
            float(envelope.max() / max(envelope.mean(), 1e-9)), 2
        ),
    }


def cmd_flag(args):
    """Flag ripple from the silhouette: the flag is bright on a black background, so its top and
    bottom edges give y(x, t). A 2-D FFT (time x position, linear trend per frame removed) peaks
    at the travelling ripple: temporal frequency and wavelength in flag widths.
    """
    frames, fps = read_frames(
        args.video, args.start, args.seconds, step=args.step, max_width=640
    )
    luma = np.array([cv2.cvtColor(f, cv2.COLOR_BGR2GRAY) for f in frames])
    inside = luma > 25
    # Columns occupied in every frame (the flag edge always exists there).
    columns = np.where(inside.any(axis=1).all(axis=0))[0]
    c0, c1 = int(columns[0]) + 4, int(columns[-1]) - 4
    flag_width = c1 - c0 + 1
    rows = np.arange(luma.shape[1])[None, :, None]
    top = np.where(inside, rows, 10**6).min(axis=1)[:, c0 : c1 + 1].astype(float)
    bottom = np.where(inside, rows, -1).max(axis=1)[:, c0 : c1 + 1].astype(float)
    results = {}
    for name, edge in (("top", top), ("bottom", bottom)):
        x = np.arange(edge.shape[1])
        detrended = np.array([e - np.polyval(np.polyfit(x, e, 1), x) for e in edge])
        detrended -= detrended.mean(axis=0)
        spec = np.fft.fft2(
            detrended
            * np.hanning(detrended.shape[0])[:, None]
            * np.hanning(detrended.shape[1])[None, :]
        )
        power = np.abs(spec) ** 2
        freqs = np.fft.fftfreq(detrended.shape[0], 1.0 / fps)
        waves = np.fft.fftfreq(
            detrended.shape[1], 1.0 / flag_width
        )  # cycles per flag width
        mask = (
            (np.abs(freqs)[:, None] >= 0.3)
            & (np.abs(freqs)[:, None] <= 6.0)
            & (np.abs(waves)[None, :] >= 0.75)
            & (np.abs(waves)[None, :] <= 6.0)
        )
        index = np.unravel_index(np.argmax(np.where(mask, power, 0)), power.shape)
        results[name] = (
            abs(float(freqs[index[0]])),
            abs(float(waves[index[1]])),
            float(detrended.std()),
        )
    f_hz = float(np.mean([r[0] for r in results.values()]))
    cycles = float(np.mean([r[1] for r in results.values()]))
    return {
        "fps": fps,
        "flag_width_px": flag_width,
        "per_edge": {
            k: {
                "hz": round(v[0], 3),
                "cycles_per_flag_width": round(v[1], 3),
                "edge_rms_px": round(v[2], 2),
            }
            for k, v in results.items()
        },
        "frequency_hz": round(f_hz, 3),
        "wavelength_flag_widths": round(1.0 / cycles, 3) if cycles > 0 else None,
        "edge_rms_flag_heights": round(
            float(
                np.mean([r[2] for r in results.values()])
                / np.mean(bottom.mean(axis=0) - top.mean(axis=0))
            ),
            4,
        ),
    }


def heat_of(bgr):
    """Flame key and heat. Key: bright and warm (red above blue, red and green both high), so the
    white-yellow core is kept while green leaves, dark rocks and grey metal fall out; holes inside
    the flame are filled by a closing. Heat: warm luminance.
    """
    b, g, r = [bgr[..., i].astype(np.float32) / 255.0 for i in range(3)]
    bright = np.clip(((r + g) * 0.5 - 0.42) * 4.0, 0, 1)
    warm = np.clip((r - b - 0.06) * 5.0, 0, 1) * np.clip((r - 0.55) * 5.0, 0, 1)
    key = bright * warm
    key = cv2.morphologyEx(
        key, cv2.MORPH_CLOSE, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (13, 13))
    )
    key = cv2.GaussianBlur(key, (0, 0), 2.0)
    heat = np.clip(0.6 * r + 0.4 * g, 0, 1)
    return heat, np.clip(key, 0, 1)


def sample_at(arrays, fps, t):
    """Linear interpolation in time between source frames."""
    x = min(max(t * fps, 0.0), len(arrays) - 1.001)
    i = int(x)
    return arrays[i] * (1.0 - (x - i)) + arrays[i + 1] * (x - i)


def cmd_flame(args):
    """Bake a looping flame atlas from a video of a fire on a dark background (ADR 0189).

    Flame pixels are keyed by colour inside ``--crop`` (red dominant, saturated, bright: the
    green leaves and dark rocks of the background fall out), heat = warm luminance, coverage =
    key. ``columns * rows`` frames are sampled at ``--rate`` frames per second from the window
    ``--start/--seconds`` (longer than needed by ``--blend`` frames); the last ``--blend`` frames
    are cross-faded with the footage that follows so that the loop has no seam. RGB = heat,
    A = coverage (FA2 coding). Also reports the flicker frequency from the flame luminance.
    """
    x, y, w, h = (int(v) for v in args.crop.split(","))
    frames, fps = read_frames(args.video, args.start, args.seconds)
    heats, covers = [], []
    for frame in frames:
        heat, key = heat_of(frame[y : y + h, x : x + w])
        heats.append(heat)
        covers.append(key)
    heats, covers = np.array(heats), np.array(covers)
    lum = (heats * covers).sum(axis=(1, 2))
    freqs, power = spectrum(lum, fps, f_min=0.5, f_max=fps / 2)
    order = np.argsort(-power)[:3]
    total = args.columns * args.rows
    blend = args.blend
    needed = (total + blend) / args.rate
    if needed > len(frames) / fps:
        raise SystemExit(
            f"window too short: {len(frames) / fps:.2f} s, need {needed:.2f} s"
        )
    sequence = []
    for i in range(total + blend):
        t = i / args.rate
        sequence.append((sample_at(heats, fps, t), sample_at(covers, fps, t)))
    out = []
    for i in range(total):
        if i < blend:
            weight = (i + 1) / (blend + 1)
            heat = weight * sequence[i][0] + (1 - weight) * sequence[total + i][0]
            cover = weight * sequence[i][1] + (1 - weight) * sequence[total + i][1]
        else:
            heat, cover = sequence[i]
        out.append((heat, cover))
    all_heat = np.concatenate([hh[c > 0.3] for hh, c in out])
    white = float(np.percentile(all_heat, 99.5)) if all_heat.size else 1.0
    px = args.frame_px
    atlas = np.zeros((args.rows * px, args.columns * px, 4), np.uint8)
    fade_rows = max(1, px // 10)
    ramp = np.ones(px, np.float32)
    ramp[-fade_rows:] = np.linspace(1, 0, fade_rows)
    for i, (hh, cover) in enumerate(out):
        side = max(hh.shape)
        canvas_h = np.zeros((side, side), np.float32)
        canvas_c = np.zeros((side, side), np.float32)
        ox = (side - hh.shape[1]) // 2
        canvas_h[side - hh.shape[0] :, ox : ox + hh.shape[1]] = hh
        canvas_c[side - hh.shape[0] :, ox : ox + hh.shape[1]] = cover
        canvas_h = cv2.resize(canvas_h, (px, px), interpolation=cv2.INTER_AREA)
        canvas_c = (
            cv2.resize(canvas_c, (px, px), interpolation=cv2.INTER_AREA) * ramp[:, None]
        )
        heat8 = (np.clip(canvas_h / white * args.heat_max, 0, 1) * 255).astype(np.uint8)
        cov8 = (np.clip(canvas_c, 0, 1) ** args.coverage_gamma * 255).astype(np.uint8)
        column, row = i % args.columns, i // args.columns
        tile = atlas[row * px : (row + 1) * px, column * px : (column + 1) * px]
        tile[..., 0] = tile[..., 1] = tile[..., 2] = heat8
        tile[..., 3] = cov8
    cv2.imwrite(args.atlas, atlas)
    loop_gap = float(np.abs(out[0][1] - sequence[total - 1][1]).mean())
    typical_gap = float(
        np.mean(
            [np.abs(out[i + 1][1] - out[i][1]).mean() for i in range(blend, total - 1)]
        )
    )
    return {
        "fps": fps,
        "playback_rate_hz": args.rate,
        "frames": total,
        "flicker_hz": [round(float(freqs[i]), 3) for i in order],
        "flicker_power_share": [round(float(power[i] / power.sum()), 3) for i in order],
        "luma_cv": round(float(lum.std() / max(lum.mean(), 1e-9)), 3),
        "white_level": round(white, 3),
        "mean_coverage": round(float(np.mean([c.mean() for _, c in out])), 4),
        "loop_gap": round(loop_gap, 4),
        "typical_frame_gap": round(typical_gap, 4),
    }


def main(argv=None):
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    sub = parser.add_subparsers(dest="command", required=True)

    def common(p):
        p.add_argument("video")
        p.add_argument("--start", type=float, default=0.0)
        p.add_argument("--seconds", type=float, default=None)
        p.add_argument("--out", default=None)

    for name in ("trebuchet", "winch"):
        p = sub.add_parser(name)
        common(p)
        p.add_argument("--pivot", required=True)
        p.add_argument("--step", type=int, default=1, help="keep one frame out of N")
        p.add_argument(
            "--override",
            default="",
            help="hand-read angles 'frame:deg,...' where the wood mask is lost against the sky",
        )
        p.add_argument("--lo-deg", type=float, default=60.0)
        p.add_argument("--hi-deg", type=float, default=235.0)
        p.add_argument("--debug", default=None)
        p.set_defaults(func=cmd_trebuchet)
    p = sub.add_parser("swing-lut")
    p.add_argument("tracked")
    p.add_argument("--first", type=int, required=True)
    p.add_argument("--last", type=int, required=True)
    p.add_argument("--points", type=int, default=25)
    p.add_argument("--out", default=None)
    p.set_defaults(func=cmd_swing_lut)
    p = sub.add_parser("cannon")
    common(p)
    p.add_argument(
        "--barrel-px",
        type=float,
        default=300.0,
        help="barrel length in a 640 px wide frame",
    )
    p.add_argument(
        "--recoil-seconds",
        type=float,
        default=0.5,
        help="window before the smoke hides the wheels",
    )
    p.set_defaults(func=cmd_cannon)
    for name, func in (("grass", cmd_grass), ("flag", cmd_flag)):
        p = sub.add_parser(name)
        common(p)
        p.add_argument("--step", type=int, default=1)
        p.add_argument(
            "--lag", type=int, default=6, help="flag: column lag of the cross-spectrum"
        )
        p.set_defaults(func=func)
    p = sub.add_parser("flame")
    common(p)
    p.add_argument("--crop", required=True)
    p.add_argument("--atlas", required=True)
    p.add_argument("--columns", type=int, default=8)
    p.add_argument("--rows", type=int, default=8)
    p.add_argument("--frame-px", type=int, default=64)
    p.add_argument(
        "--rate", type=float, default=35.0, help="frames per second of the sampled loop"
    )
    p.add_argument(
        "--blend", type=int, default=8, help="frames cross-faded to close the loop"
    )
    p.add_argument("--heat-max", type=float, default=0.85)
    p.add_argument("--coverage-gamma", type=float, default=0.6)
    p.set_defaults(func=cmd_flame)
    args = parser.parse_args(argv)
    result = args.func(args)
    text = json.dumps(result)
    if args.out:
        with open(args.out, "w") as handle:
            handle.write(text)
    print(text)


if __name__ == "__main__":
    sys.exit(main())
