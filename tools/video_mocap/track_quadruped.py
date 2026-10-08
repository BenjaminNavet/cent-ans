# /// script
# requires-python = ">=3.10,<3.13"
# dependencies = ["opencv-python-headless>=4.9", "numpy>=1.26"]
# ///
"""Lot AS8b: gaits of a quadruped (horse) from side views of free films.

No animal pose model is used (ADR 0187, ADR 0189): the key points are posed by hand on a few
frames (``tools/video_mocap/data/horse_keypoints.json``: root of the forelegs ``S`` and of the
hind legs ``H``, the four hooves ``F1 F2 H1 H2``), optionally tracked between those frames
(Lucas-Kanade, ``track``), then folded onto one stride and smoothed (Fourier series). The
output gives, per leg, the hoof in the sagittal plane relative to the root of the leg, in
leg lengths (``u`` forward, ``d`` down), the joint angles of a two-segment leg, the stance
(hoof on the ground) of each frame and the movement of the body (rise, pitch). Videos and
frames stay outside the repository::

    uv run tools/video_mocap/track_quadruped.py fit KEYPOINTS.json GAIT OUT.json [--harmonics 3]
    uv run tools/video_mocap/track_quadruped.py track FRAMES_DIR KEYPOINTS.json GAIT --seeds 0 3 6 9

``fit``: GAIT = ``gallop`` or ``trot`` (key of the file). ``track``: seeds the key points on
the given frames (indices from 0), tracks them over the others with
``cv2.calcOpticalFlowPyrLK`` (forward from the previous seed and backward from the next one,
mixed by the distance) and prints the mean error against the hand-posed points, which
measures the tracker (frames with fast legs are lost: the hand-posed points are kept).
"""

import argparse
import json
import math
import sys
from pathlib import Path

import numpy as np

LEGS = ("F1", "F2", "H1", "H2")
SEG_FORE = 0.52  # upper segment / leg length (shoulder-knee), lower = 1 - this + slack
SEG_HIND = 0.50
SLACK = 0.06  # sum of the two segments exceeds the standing leg length by this share
PLANTED = 0.07  # a hoof within this many leg lengths of the ground is planted


def _frames_px(gait):
    """(n, 6 points, 2) source pixels: S, H, F1, F2, H1, H2, and the ground y per frame."""
    sc = gait["view"]["scale"]
    ox, oy = gait["view"]["origin"]
    out = []
    ground = []
    for fr in gait["frames"]:
        out.append(
            [
                (ox + fr[k][0] / sc, oy + fr[k][1] / sc)
                for k in ("S", "H", "F1", "F2", "H1", "H2")
            ]
        )
        g = fr.get("ground", gait.get("ground_y"))
        ground.append(oy + g / sc)
    return np.array(out, dtype=float), np.array(ground, dtype=float)


def normalise(gait):
    """Per frame: hoof offsets (u, d) in leg lengths, planted flags, body rise and pitch."""
    pts, ground = _frames_px(gait)
    root_mid = (pts[:, 0, 1] + pts[:, 1, 1]) / 2.0
    length = float(np.median(ground - root_mid))
    n = len(pts)
    u = np.zeros((n, 4))
    d = np.zeros((n, 4))
    height = np.zeros((n, 4))  # hoof above the ground, in leg lengths
    for j, _leg in enumerate(LEGS):
        root = pts[:, 0 if j < 2 else 1]
        hoof = pts[:, 2 + j]
        u[:, j] = (hoof[:, 0] - root[:, 0]) / length
        d[:, j] = (hoof[:, 1] - root[:, 1]) / length
        height[:, j] = (ground - hoof[:, 1]) / length
    rise = (ground - root_mid) / length
    pitch = np.arctan2(pts[:, 1, 1] - pts[:, 0, 1], pts[:, 0, 0] - pts[:, 1, 0])
    return {
        "length_px": length,
        "u": u,
        "d": d,
        "height": height,
        "planted": height < gait.get("planted_tol", PLANTED),
        "rise": rise,
        "pitch": pitch,
    }


def _design(phase, harmonics):
    cols = [np.ones_like(phase)]
    for k in range(1, harmonics + 1):
        cols.append(np.cos(2 * math.pi * k * phase))
        cols.append(np.sin(2 * math.pi * k * phase))
    return np.stack(cols, axis=1)


def fourier_fit(phase, values, harmonics):
    """Coefficients of the periodic least-squares fit, and its residual rms."""
    a = _design(phase, harmonics)
    coef, *_ = np.linalg.lstsq(a, values, rcond=None)
    rms = float(np.sqrt(np.mean((a @ coef - values) ** 2)))
    return coef, rms


def best_period(norm, nominal, harmonics):
    """Stride length in frames (around `nominal`) that folds the frames best."""
    n = len(norm["u"])
    best = (1e9, nominal)
    for p in np.arange(nominal - 0.8, nominal + 0.8, 0.02):
        phase = (np.arange(n) / p) % 1.0
        cost = 0.0
        for arr in (norm["u"], norm["d"]):
            for j in range(4):
                cost += fourier_fit(phase, arr[:, j], harmonics)[1]
        best = min(best, (cost, float(p)))
    return best[1]


def leg_angles(u, d, fore):
    """Angles (rad from straight down, forward positive) of the two segments of a leg.

    The hoof is at (u, d) from the root (leg lengths); the segments are fixed shares of
    the standing leg length; the joint (knee of the foreleg, hock of the hind leg) bends
    forward for the fore, backward for the hind.
    """
    a = (SEG_FORE if fore else SEG_HIND) * (1.0 + SLACK)
    b = (1.0 - (SEG_FORE if fore else SEG_HIND)) * (1.0 + SLACK)
    dist = max(math.hypot(u, d), 1e-6)
    dist = min(dist, (a + b) * 0.999)
    cos_root = (a * a + dist * dist - b * b) / (2 * a * dist)
    root_off = math.acos(max(-1.0, min(1.0, cos_root)))
    base = math.atan2(u, d)  # direction root -> hoof from straight down
    sign = 1.0 if fore else -1.0  # joint offset towards the front (fore) or the back (hind)
    upper = base + sign * root_off
    kx = a * math.sin(upper)
    kd = a * math.cos(upper)
    lower = math.atan2(u * dist / max(dist, 1e-6) - kx, d - kd)
    return upper, lower


def fit(keypoints, gait_name, harmonics, samples, refine):
    gait = keypoints[gait_name]
    norm = normalise(gait)
    n = len(norm["u"])
    period = float(gait["period_frames"])
    if refine:
        period = best_period(norm, period, min(harmonics, 2))
    phase = (np.arange(n) / period) % 1.0
    grid = np.arange(samples) / samples
    design = _design(grid, harmonics)
    curves = {}
    rms = {}
    for name in ("u", "d"):
        for j, leg in enumerate(LEGS):
            coef, r = fourier_fit(phase, norm[name][:, j], harmonics)
            curves.setdefault(leg, {})[name] = (design @ coef).tolist()
            rms[f"{leg}.{name}"] = r
    if gait.get("symmetric"):
        # Trot and walk: the second leg of a pair is the first one half a stride later.
        half = samples // 2
        for a, b in (("F1", "F2"), ("H1", "H2")):
            for name in ("u", "d"):
                ca = np.array(curves[a][name])
                cb = np.array(curves[b][name])
                mean_a = (ca + np.roll(cb, half)) / 2.0  # b(phi) = a(phi + 0.5)
                curves[a][name] = mean_a.tolist()
                curves[b][name] = np.roll(mean_a, -half).tolist()
    for name in ("rise", "pitch"):
        k_body = min(harmonics, 2)
        coef, r = fourier_fit(phase, norm[name], k_body)
        curves[name] = (_design(grid, k_body) @ coef).tolist()
        rms[name] = r
    mean_rise = float(np.mean(curves["rise"]))
    curves["rise"] = [v - mean_rise for v in curves["rise"]]
    # Stance of each leg on the folded frames: fraction planted, and its phase window.
    stance = {}
    for j, leg in enumerate(LEGS):
        flags = norm["planted"][:, j]
        phases = phase[flags]
        stance[leg] = {
            "duty": float(flags.mean()),
            "planted_phases": sorted(round(float(p), 3) for p in phases),
        }
    joints = {}
    for j, leg in enumerate(LEGS):
        fore = j < 2
        up, lo = zip(
            *(
                leg_angles(curves[leg]["u"][i], curves[leg]["d"][i], fore)
                for i in range(samples)
            ),
            strict=True,
        )
        joints[leg] = {"upper": list(up), "lower": list(lo)}
    # Where does each hoof touch down? phase of the fitted maximum reach (u) of each leg.
    reach = {leg: float(np.argmax(curves[leg]["u"]) / samples) for leg in LEGS}
    return {
        "gait": gait_name,
        "source": gait["source"],
        "frames": n,
        "period_frames": period,
        "samples": samples,
        "harmonics": harmonics,
        "leg_length_px": norm["length_px"],
        "legs": {leg: curves[leg] for leg in LEGS},
        "joints": joints,
        "rise": curves["rise"],
        "pitch": curves["pitch"],
        "stance": stance,
        "max_reach_phase": reach,
        "fit_rms": rms,
    }


def track(frames_dir, keypoints, gait_name, seeds):
    """Track the hand-posed points from `seeds` over the other frames and report the error."""
    import cv2

    gait = keypoints[gait_name]
    pts, _ground = _frames_px(gait)
    paths = sorted(Path(frames_dir).glob("f*.png"))
    grays = [cv2.cvtColor(cv2.imread(str(p)), cv2.COLOR_BGR2GRAY) for p in paths]
    n = len(grays)
    lk = {"winSize": (31, 31), "maxLevel": 4, "criteria": (3, 30, 0.01)}

    def run(start, stop, step):
        cur = pts[start].astype(np.float32).reshape(-1, 1, 2)
        out = {start: cur.reshape(-1, 2).copy()}
        i = start
        while i != stop:
            nxt, st, _err = cv2.calcOpticalFlowPyrLK(grays[i], grays[i + step], cur, None, **lk)
            cur = np.where(st == 1, nxt, cur)
            i += step
            out[i] = cur.reshape(-1, 2).copy()
        return out

    seeds = sorted(seeds)
    errors = []
    for a, b in zip(seeds, seeds[1:] + [seeds[0] + n], strict=True):
        fwd = run(a, min(b, n - 1), 1) if b < n else run(a, n - 1, 1)
        bwd = run(b % n, a if b < n else 0, -1) if b < n else {}
        for i in range(a + 1, min(b, n)):
            if i in seeds:
                continue
            w = (i - a) / (b - a)
            est = fwd[i] if i not in bwd else (1 - w) * fwd[i] + w * bwd[i]
            errors.append(np.linalg.norm(est - pts[i], axis=1))
    err = np.array(errors)
    norm_len = normalise(gait)["length_px"]
    print(f"tracked frames: {len(err)}; mean error {err.mean():.1f} px "
          f"({err.mean() / norm_len:.2f} leg lengths); hooves {err[:, 2:].mean():.1f} px; "
          f"roots {err[:, :2].mean():.1f} px")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = parser.add_subparsers(dest="cmd", required=True)
    p_fit = sub.add_parser("fit")
    p_fit.add_argument("keypoints")
    p_fit.add_argument("gait")
    p_fit.add_argument("out")
    p_fit.add_argument("--harmonics", type=int, default=3)
    p_fit.add_argument("--samples", type=int, default=48)
    p_fit.add_argument("--refine-period", action="store_true")
    p_trk = sub.add_parser("track")
    p_trk.add_argument("frames_dir")
    p_trk.add_argument("keypoints")
    p_trk.add_argument("gait")
    p_trk.add_argument("--seeds", type=int, nargs="+", required=True)
    args = parser.parse_args(argv)
    keypoints = json.loads(Path(args.keypoints).read_text())
    if args.cmd == "fit":
        result = fit(keypoints, args.gait, args.harmonics, args.samples, args.refine_period)
        Path(args.out).write_text(json.dumps(result, indent=1))
        print(
            f"{args.gait}: period {result['period_frames']:.2f} frames, "
            f"duty {[round(result['stance'][k]['duty'], 2) for k in LEGS]}, "
            f"rms {max(result['fit_rms'].values()):.3f}"
        )
    else:
        track(args.frames_dir, keypoints, args.gait, args.seeds)
    return 0


if __name__ == "__main__":
    sys.exit(main())
