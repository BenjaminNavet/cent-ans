# /// script
# requires-python = ">=3.10,<3.13"
# dependencies = ["opencv-python-headless>=4.9", "numpy>=1.26"]
# ///
"""Lot AS8c: measure animal and cart motion in free videos (optical tracking of placed points).

The videos stay outside the repository (ADR 0189, ``~/dev/cent-ans-mocap-src/video/free/``);
only the measured numbers are versioned (``data/fx/animal_motion_measured.json``).

Sub-commands::

    uv run tools/video_mocap/measure_motion.py sheet VIDEO OUT.png --start 5 --step 0.5 --n 12
    uv run tools/video_mocap/measure_motion.py track VIDEO POINTS.json OUT.npz
    uv run tools/video_mocap/measure_motion.py analyse OUT.npz POINTS.json

``sheet`` writes a contact sheet of frames with a coordinate grid in source pixels, to pick the
points by eye. ``POINTS.json``: ``{"start_s": 5.0, "end_s": 12.0, "scale": 1.0, "points":
{"hoof_f": [x, y], ...}}`` -- pixel coordinates in the frame at ``start_s``. ``track`` follows
each point with pyramidal Lucas-Kanade (backward-forward check, a lost point is held and
flagged). ``analyse`` turns the trajectories into frequencies, amplitudes, phases and harmonic
content; its JSON output is what ``data/fx/animal_motion_measured.json`` is built from.

Image y is down; every vertical quantity below is reported upward (``-y``). Distances are
normalised by a body length given in ``POINTS.json`` (``body_px``: length of the animal or cart
in pixels at ``start_s``) so the result is unit-free (fractions of body length).
"""

import argparse
import json
import sys
from pathlib import Path

import cv2
import numpy as np

LK = {
    "winSize": (21, 21),
    "maxLevel": 3,
    "criteria": (cv2.TERM_CRITERIA_EPS | cv2.TERM_CRITERIA_COUNT, 30, 0.01),
}


def read_frame(cap, t):
    """Read frame."""
    cap.set(cv2.CAP_PROP_POS_MSEC, t * 1000.0)
    ok, frame = cap.read()
    return frame if ok else None


def cmd_sheet(args):
    """Cmd sheet."""
    cap = cv2.VideoCapture(args.video)
    tiles = []
    for i in range(args.n):
        t = args.start + i * args.step
        frame = read_frame(cap, t)
        if frame is None:
            break
        if args.crop:
            x0, y0, x1, y1 = args.crop
            frame = frame[y0:y1, x0:x1]
        else:
            x0, y0 = 0, 0
        scale = args.width / frame.shape[1]
        small = cv2.resize(frame, None, fx=scale, fy=scale)
        for gx in range(0, frame.shape[1], args.grid):
            cv2.line(
                small,
                (int(gx * scale), 0),
                (int(gx * scale), small.shape[0]),
                (0, 255, 255),
                1,
            )
            cv2.putText(
                small,
                str(gx + x0),
                (int(gx * scale) + 2, 10),
                cv2.FONT_HERSHEY_SIMPLEX,
                0.3,
                (0, 255, 255),
                1,
            )
        for gy in range(0, frame.shape[0], args.grid):
            cv2.line(
                small,
                (0, int(gy * scale)),
                (small.shape[1], int(gy * scale)),
                (255, 255, 0),
                1,
            )
            cv2.putText(
                small,
                str(gy + y0),
                (2, int(gy * scale) - 2),
                cv2.FONT_HERSHEY_SIMPLEX,
                0.3,
                (255, 255, 0),
                1,
            )
        cv2.putText(
            small,
            f"t={t:.2f}",
            (small.shape[1] - 70, 12),
            cv2.FONT_HERSHEY_SIMPLEX,
            0.4,
            (0, 0, 255),
            1,
        )
        tiles.append(small)
    if not tiles:
        sys.exit("no frame read")
    cols = args.cols
    while len(tiles) % cols:
        tiles.append(np.zeros_like(tiles[0]))
    rows = [np.hstack(tiles[r : r + cols]) for r in range(0, len(tiles), cols)]
    cv2.imwrite(args.out, np.vstack(rows))


def cmd_track(args):
    """Cmd track."""
    spec = json.loads(Path(args.points).read_text())
    cap = cv2.VideoCapture(args.video)
    fps = cap.get(cv2.CAP_PROP_FPS)
    cap.set(cv2.CAP_PROP_POS_MSEC, spec["start_s"] * 1000.0)
    names = list(spec["points"])
    pts = np.array([spec["points"][n] for n in names], dtype=np.float32).reshape(
        -1, 1, 2
    )
    ok, frame = cap.read()
    prev = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)
    # Points listed in ``mil`` ({name: box size in px}) use an appearance tracker (hooves, heads)
    # instead of Lucas-Kanade, which slips on small high-contrast features.
    mil = {}
    for name, box in spec.get("mil", {}).items():
        x, y = spec["points"][name]
        tracker = cv2.TrackerMIL_create()
        tracker.init(frame, (int(x - box / 2), int(y - box / 2), int(box), int(box)))
        mil[names.index(name)] = tracker
    traj = [pts.reshape(-1, 2).copy()]
    lost = [np.zeros(len(names), bool)]
    times = [spec["start_s"]]
    while True:
        t = spec["start_s"] + len(traj) / fps
        if t > spec["end_s"]:
            break
        ok, frame = cap.read()
        if not ok:
            break
        gray = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)
        nxt, st, _ = cv2.calcOpticalFlowPyrLK(prev, gray, pts, None, **LK)
        back, st2, _ = cv2.calcOpticalFlowPyrLK(gray, prev, nxt, None, **LK)
        err = np.linalg.norm((back - pts).reshape(-1, 2), axis=1)
        good = (st.ravel() == 1) & (st2.ravel() == 1) & (err < 1.5)
        new = np.where(
            good[:, None, None],
            nxt,
            pts + (pts - (pts if len(traj) < 2 else traj[-2].reshape(-1, 1, 2))),
        )
        pts = new.astype(np.float32)
        for i, tracker in mil.items():
            found, rect = tracker.update(frame)
            good[i] = bool(found)
            if found:
                pts[i, 0] = (rect[0] + rect[2] / 2, rect[1] + rect[3] / 2)
        traj.append(pts.reshape(-1, 2).copy())
        lost.append(~good)
        times.append(t)
        prev = gray
    np.savez(
        args.out,
        names=np.array(names),
        traj=np.array(traj),
        lost=np.array(lost),
        times=np.array(times),
        fps=fps,
    )
    lost_frac = np.array(lost).mean(axis=0)
    print(
        "frames",
        len(traj),
        "lost fraction",
        dict(zip(names, np.round(lost_frac, 2).tolist(), strict=True)),
    )


def cmd_overlay(args):
    """Contact sheet of tracked points drawn on evenly spaced frames, to check the tracking."""
    d = np.load(args.npz)
    cap = cv2.VideoCapture(args.video)
    names = [str(n) for n in d["names"]]
    tiles = []
    for k in np.linspace(0, len(d["traj"]) - 1, args.n).astype(int):
        frame = read_frame(cap, float(d["times"][k]))
        if frame is None:
            continue
        x0, y0, x1, y1 = args.crop
        scale = args.width / (x1 - x0)
        small = cv2.resize(frame[y0:y1, x0:x1], None, fx=scale, fy=scale)
        for i, name in enumerate(names):
            x, y = (d["traj"][k, i] - (x0, y0)) * scale
            colour = (0, 0, 255) if d["lost"][k, i] else (0, 255, 0)
            cv2.circle(small, (int(x), int(y)), 3, colour, -1)
            cv2.putText(
                small,
                name,
                (int(x) + 4, int(y) - 3),
                cv2.FONT_HERSHEY_SIMPLEX,
                0.35,
                (255, 255, 0),
                1,
            )
        cv2.putText(
            small,
            f"t={d['times'][k]:.2f}",
            (4, 14),
            cv2.FONT_HERSHEY_SIMPLEX,
            0.45,
            (0, 0, 255),
            1,
        )
        tiles.append(small)
    while len(tiles) % args.cols:
        tiles.append(np.zeros_like(tiles[0]))
    rows = [
        np.hstack(tiles[r : r + args.cols]) for r in range(0, len(tiles), args.cols)
    ]
    cv2.imwrite(args.out, np.vstack(rows))


def cmd_period(args):
    """Period of a repeating motion from the self-similarity of a body-stabilised image band.

    ``POINTS.json``: ``start_s``, ``end_s``, ``box`` [x, y, w, h] around the animal at ``start_s``,
    ``band`` [y0, y1] fractions of the box height (legs: [0.55, 1.0]). The box is followed by an
    appearance tracker, the band is cut at its centre, and the mean absolute difference between
    frames ``lag`` apart is computed; the first deep minimum is the cycle period.
    """
    spec = json.loads(Path(args.points).read_text())
    cap = cv2.VideoCapture(args.video)
    fps = cap.get(cv2.CAP_PROP_FPS)
    cap.set(cv2.CAP_PROP_POS_MSEC, spec["start_s"] * 1000.0)
    ok, frame = cap.read()
    x, y, w, h = spec["box"]
    tracker = cv2.TrackerMIL_create()
    tracker.init(frame, (x, y, w, h))
    b0, b1 = spec.get("band", [0.55, 1.0])
    bands = []
    centres = []
    while (len(bands) / fps) < spec["end_s"] - spec["start_s"]:
        gray = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)
        found, rect = tracker.update(frame)
        if found:
            rx, ry, rw, rh = (int(v) for v in rect)
        ya, yb = ry + int(rh * b0), ry + int(rh * b1)
        cx = int(spec.get("anchor_x", 0.5) * rw) + rx
        half = int(w * spec.get("half_width", 0.5))
        patch = gray[max(ya, 0) : yb, max(cx - half, 0) : cx + half]
        if patch.size == 0:
            break
        bands.append(cv2.resize(patch, (64, 24)).astype(np.float32))
        centres.append((rx + rw / 2, ry + rh / 2))
        ok, frame = cap.read()
        if not ok:
            break
    stack = np.array(bands)
    stack -= stack.mean(axis=(1, 2), keepdims=True)
    n = len(stack)
    max_lag = min(n // 2, int(spec.get("max_lag_s", 2.5) * fps))
    curve = np.array(
        [np.mean(np.abs(stack[: n - lag] - stack[lag:])) for lag in range(1, max_lag)]
    )
    curve /= curve.max() + 1e-9
    lags = np.arange(1, max_lag) / fps
    mins = [
        k
        for k in range(1, len(curve) - 1)
        if curve[k] < curve[k - 1]
        and curve[k] <= curve[k + 1]
        and lags[k] > spec.get("min_period_s", 0.4)
    ]
    result = {
        "frames": n,
        "fps": fps,
        "minima": [
            (round(float(lags[k]), 3), round(float(curve[k]), 3)) for k in mins[:4]
        ],
    }
    if mins:
        result["period_s"] = round(float(lags[mins[0]]), 3)
        result["hz"] = round(1.0 / lags[mins[0]], 3)
    print(json.dumps(result))
    if args.json:
        with open(args.json, "w") as handle:
            json.dump(
                {
                    **result,
                    "curve": [round(float(v), 4) for v in curve],
                    "centres": [(round(a, 1), round(b, 1)) for a, b in centres],
                },
                handle,
            )


def detrend(signal, fps, seconds=1.5):
    """Remove the slow drift (camera pan, slope) with a moving average of ``seconds``."""
    win = max(3, int(seconds * fps))
    pad = np.pad(signal, win // 2, mode="edge")
    slow = np.convolve(pad, np.ones(win) / win, mode="same")[
        win // 2 : win // 2 + len(signal)
    ]
    return signal - slow


def dominant_hz(signal, fps, fmin, fmax):
    """Dominant frequency (Hz) of a signal inside [fmin, fmax] (zero-padded Hann FFT)."""
    x = signal - np.mean(signal)
    n = len(x)
    pad = 1 << (n * 8 - 1).bit_length()
    spec = np.abs(np.fft.rfft(x * np.hanning(n), pad))
    freqs = np.fft.rfftfreq(pad, 1.0 / fps)
    return float(freqs[np.argmax(spec * ((freqs >= fmin) & (freqs <= fmax)))])


def fit_harmonics(signal, fps, f0, count=3):
    """Least-squares ``sum a_h cos(2 pi h f0 t) + b_h sin(...)``: [(amplitude, phase_cycles)].

    The phase is that of the cosine form ``A cos(2 pi (h f0 t - phase))``, in cycles of the
    fundamental (0..1), so phases of different signals at the same ``f0`` compare directly.
    """
    n = len(signal)
    t = np.arange(n) / fps
    cols = [np.ones(n)]
    for h in range(1, count + 1):
        cols += [np.cos(2 * np.pi * f0 * h * t), np.sin(2 * np.pi * f0 * h * t)]
    design = np.array(cols).T
    coef, *_ = np.linalg.lstsq(design, signal, rcond=None)
    out = []
    for h in range(count):
        a, b = coef[1 + 2 * h], coef[2 + 2 * h]
        out.append(
            (float(np.hypot(a, b)), float((np.arctan2(b, a) / (2 * np.pi)) % 1.0))
        )
    resid = float(np.std(signal - design @ coef) / (np.std(signal) + 1e-12))
    return out, resid


def fold_profile(signal, fps, f0, phase0, bins=16):
    """Cycle-averaged waveform: ``bins`` means of the signal folded on ``f0`` (phase 0 = ``phase0``)."""
    phase = (np.arange(len(signal)) / fps * f0 - phase0) % 1.0
    index = np.minimum((phase * bins).astype(int), bins - 1)
    out = np.full(bins, np.nan)
    for k in range(bins):
        sel = signal[index == k]
        if len(sel):
            out[k] = np.mean(sel)
    if np.isnan(out).all():
        return [0.0] * bins
    ok = ~np.isnan(out)
    xs = np.arange(bins)
    out = np.interp(xs, xs[ok], out[ok], period=bins)
    return [round(float(v), 4) for v in out - np.mean(out)]


def describe(signal, fps, f0, label, phase0=0.0):
    """Describe."""
    harm, resid = fit_harmonics(signal, fps, f0)
    lo, hi = np.percentile(signal, [5, 95])
    return {
        "amp_h1": round(harm[0][0], 4),
        "phase_h1": round(harm[0][1], 3),
        "amp_h2": round(harm[1][0], 4),
        "phase_h2": round(harm[1][1], 3),
        "amp_h3": round(harm[2][0], 4),
        "phase_h3": round(harm[2][1], 3),
        "p5_p95_half": round(float((hi - lo) / 2), 4),
        "residual_share": round(resid, 3),
        "own_hz": round(dominant_hz(signal, fps, 0.2, min(6.0, fps / 2 - 0.5)), 3),
        "profile16": fold_profile(signal, fps, f0, phase0),
        "label": label,
    }


def cmd_analyse(args):
    """Cmd analyse."""
    d = np.load(args.npz)
    spec = json.loads(Path(args.points).read_text())
    names = [str(n) for n in d["names"]]
    traj = d["traj"].astype(float)
    fps = float(d["fps"])
    body = float(spec["body_px"])
    ref = names.index(spec["reference"]) if spec.get("reference") else None
    bg = names.index(spec["background"]) if spec.get("background") else None
    legs = spec.get("legs", [])
    out = {
        "video_fps": fps,
        "frames": len(traj),
        "body_px": body,
        "points": {},
        "angles": {},
    }

    def x_of(i):
        return traj[:, i, 0] / body

    def y_of(i):
        return -traj[:, i, 1] / body

    # Gait fundamental from the first leg, relative to the reference point.
    fmin, fmax = spec.get("fmin", 0.3), spec.get("fmax", 3.0)
    f0 = spec.get("f0")
    if f0 is None:
        li = names.index(legs[0])
        f0 = dominant_hz(detrend(x_of(li) - x_of(ref), fps), fps, fmin, fmax)
    out["f0_hz"] = round(f0, 3)
    # Phase origin: the first listed signal peaks at phase 0 (a leg: hoof furthest forward).
    phase0 = 0.0
    if legs:
        first = names.index(legs[0])
        sig = detrend(x_of(first) - (x_of(ref) if ref is not None else 0.0), fps)
        phase0 = fit_harmonics(sig, fps, f0)[0][0][1]
    out["phase0"] = round(phase0, 3)
    for i, name in enumerate(names):
        entry = {}
        if ref is not None and i != ref:
            rel_x = detrend(x_of(i) - x_of(ref), fps)
            rel_y = detrend(y_of(i) - y_of(ref), fps)
            entry["x_rel_ref"] = describe(rel_x, fps, f0, "x - ref", phase0)
            entry["y_rel_ref"] = describe(rel_y, fps, f0, "y - ref", phase0)
        wy = y_of(i) - (y_of(bg) if bg is not None else 0.0)
        entry["y_world"] = describe(detrend(wy, fps), fps, f0, "y - background", phase0)
        out["points"][name] = entry
    for key, (a, b) in spec.get("angles", {}).items():
        ia, ib = names.index(a), names.index(b)
        ang = np.unwrap(
            np.arctan2(
                -(traj[:, ib, 1] - traj[:, ia, 1]), traj[:, ib, 0] - traj[:, ia, 0]
            )
        )
        ang = detrend(ang, fps, 3.0)
        info = describe(ang, fps, f0, f"angle {a}->{b} (rad, image plane)", phase0)
        info["min_rad"], info["max_rad"] = (
            round(float(v), 3) for v in np.percentile(ang, [2, 98])
        )
        out["angles"][key] = info
    grounds = spec.get("ground") or []
    if isinstance(grounds, str):
        grounds = [grounds]
    if grounds and ref is not None:
        # Speed of the reference relative to ground texture points (median of the slopes).
        times = np.arange(len(traj)) / fps
        slopes = [
            np.polyfit(times, x_of(ref) - x_of(names.index(g)), 1)[0] for g in grounds
        ]
        slope = float(np.median(slopes))
        out["speed_body_per_s"] = round(slope, 4)
        out["stride_body"] = round(abs(slope) / f0, 4) if f0 else None
    print(json.dumps(out, indent=1))
    if args.json:
        Path(args.json).write_text(json.dumps(out, indent=1))


def main():
    """Main."""
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    sub = ap.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("sheet")
    s.add_argument("video")
    s.add_argument("out")
    s.add_argument("--start", type=float, default=0.0)
    s.add_argument("--step", type=float, default=1.0)
    s.add_argument("--n", type=int, default=6)
    s.add_argument("--cols", type=int, default=2)
    s.add_argument(
        "--grid", type=int, default=100, help="grid spacing in source pixels"
    )
    s.add_argument("--width", type=int, default=320)
    s.add_argument("--crop", type=int, nargs=4, metavar=("X0", "Y0", "X1", "Y1"))
    s.set_defaults(fn=cmd_sheet)
    t = sub.add_parser("track")
    t.add_argument("video")
    t.add_argument("points")
    t.add_argument("out")
    t.set_defaults(fn=cmd_track)
    p = sub.add_parser("period")
    p.add_argument("video")
    p.add_argument("points")
    p.add_argument("--json")
    p.set_defaults(fn=cmd_period)
    o = sub.add_parser("overlay")
    o.add_argument("video")
    o.add_argument("npz")
    o.add_argument("out")
    o.add_argument("--n", type=int, default=6)
    o.add_argument("--cols", type=int, default=2)
    o.add_argument("--width", type=int, default=320)
    o.add_argument(
        "--crop", type=int, nargs=4, required=True, metavar=("X0", "Y0", "X1", "Y1")
    )
    o.set_defaults(fn=cmd_overlay)
    a = sub.add_parser("analyse")
    a.add_argument("npz")
    a.add_argument("points")
    a.add_argument("--json")
    a.set_defaults(fn=cmd_analyse)
    args = ap.parse_args()
    args.fn(args)


if __name__ == "__main__":
    main()
