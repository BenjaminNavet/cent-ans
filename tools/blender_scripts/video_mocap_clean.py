"""Lot NT13: clean-up of video pose landmarks (pure numpy, no Blender, tested by pytest).

Input: MediaPipe Pose Landmarker world landmarks (33 points, metres, hip-centred, x right of
the image, y down, z away from the camera) and normalised image landmarks, one row per video
frame (``tools/video_mocap/extract_pose.py``). Output, for ``nt13_video_trial.py``: landmark
positions in a Z-up frame with a root trajectory, smoothed, with constant segment lengths,
resampled to the clip rate, plus per-foot ground contacts.

Steps (``clean``):

1. ``to_zup``: MediaPipe axes to X right, Y away from the camera, Z up (right handed).
2. ``one_euro``: One-Euro filter (Casiez et al. 2012) run forwards then backwards (no lag);
   a low cut-off where the joint is slow, a higher one where it is fast (strikes keep their
   snap, the stance keeps still).
3. ``enforce_lengths``: every limb segment gets its median length; the in-image offset of the
   child is kept and only the depth (Y) is solved, keeping the sign of the measured depth
   (MediaPipe's depth is its weakest axis, foreshortened limbs come out too short).
4. ``foot_contacts``: a foot is planted while it is (nearly) the lowest one in the image and
   slow in the image (camera still); short gaps are filled and short runs dropped.
5. ``root_trajectory``: the hip-centred pose gets a root motion driven by the planted feet
   (the root moves opposite to the planted foot's motion relative to the hips) and a height
   putting the lowest foot on the ground.
6. ``resample``: linear resampling (video 30 fps to clips 24 fps).

``pin_targets`` and ``two_bone_ik`` serve the foot locking done on the target rig.
"""

import numpy as np

# MediaPipe Pose landmark indices ("L" = the performer's left).
NOSE = 0
EAR_L, EAR_R = 7, 8
SHOULDER_L, SHOULDER_R = 11, 12
ELBOW_L, ELBOW_R = 13, 14
WRIST_L, WRIST_R = 15, 16
PINKY_L, PINKY_R = 17, 18
INDEX_L, INDEX_R = 19, 20
THUMB_L, THUMB_R = 21, 22
HIP_L, HIP_R = 23, 24
KNEE_L, KNEE_R = 25, 26
ANKLE_L, ANKLE_R = 27, 28
HEEL_L, HEEL_R = 29, 30
TOE_L, TOE_R = 31, 32

# (parent, child) limb segments, parents before children. The torso (shoulders, hips) and the
# head are left as measured.
SEGMENTS = (
    (SHOULDER_L, ELBOW_L),
    (ELBOW_L, WRIST_L),
    (WRIST_L, PINKY_L),
    (WRIST_L, INDEX_L),
    (WRIST_L, THUMB_L),
    (SHOULDER_R, ELBOW_R),
    (ELBOW_R, WRIST_R),
    (WRIST_R, PINKY_R),
    (WRIST_R, INDEX_R),
    (WRIST_R, THUMB_R),
    (HIP_L, KNEE_L),
    (KNEE_L, ANKLE_L),
    (ANKLE_L, HEEL_L),
    (ANKLE_L, TOE_L),
    (HIP_R, KNEE_R),
    (KNEE_R, ANKLE_R),
    (ANKLE_R, HEEL_R),
    (ANKLE_R, TOE_R),
)

FEET = ((ANKLE_L, HEEL_L, TOE_L), (ANKLE_R, HEEL_R, TOE_R))


def to_zup(world):
    """MediaPipe world axes (x right, y down, z away) to X right, Y away, Z up."""
    w = np.asarray(world, dtype=np.float64)
    return np.stack((w[..., 0], w[..., 2], -w[..., 1]), axis=-1)


def _alpha(cutoff, rate):
    """Smoothing factor of an exponential filter of cut-off `cutoff` Hz at `rate` Hz."""
    tau = 1.0 / (2.0 * np.pi * cutoff)
    return 1.0 / (1.0 + tau * rate)


def _one_euro_pass(x, rate, min_cutoff, beta, d_cutoff):
    out = np.empty_like(x)
    out[0] = x[0]
    dx_hat = np.zeros_like(x[0])
    a_d = _alpha(d_cutoff, rate)
    for t in range(1, len(x)):
        dx = (x[t] - out[t - 1]) * rate
        dx_hat = a_d * dx + (1.0 - a_d) * dx_hat
        a = _alpha(min_cutoff + beta * np.abs(dx_hat), rate)
        out[t] = a * x[t] + (1.0 - a) * out[t - 1]
    return out


def one_euro(x, rate, min_cutoff=1.5, beta=0.3, d_cutoff=1.0, zero_phase=True):
    """One-Euro filter of `x` (frames first) sampled at `rate` Hz.

    `zero_phase` runs it forwards then backwards over the result (offline: no lag).
    """
    x = np.asarray(x, dtype=np.float64)
    if len(x) < 2:
        return x.copy()
    out = _one_euro_pass(x, rate, min_cutoff, beta, d_cutoff)
    if zero_phase:
        out = _one_euro_pass(out[::-1], rate, min_cutoff, beta, d_cutoff)[::-1]
    return np.ascontiguousarray(out)


def segment_lengths(points, segments=SEGMENTS):
    """Median length of every segment over the clip (dict (parent, child) -> metres)."""
    return {
        (a, b): float(np.median(np.linalg.norm(points[:, b] - points[:, a], axis=-1)))
        for a, b in segments
    }


def enforce_lengths(points, segments=SEGMENTS, lengths=None):
    """Constant segment lengths by solving the depth (Y) of each child (Z-up points).

    The child keeps its X and Z offset to its (already corrected) parent; its Y offset is set
    so that the segment has its median length, with the measured sign. An in-image offset
    already longer than the segment is scaled down to it (Y offset 0).
    """
    pts = np.array(points, dtype=np.float64)
    lengths = lengths or segment_lengths(pts, segments)
    orig = np.array(pts)
    for a, b in segments:
        off = orig[:, b] - orig[:, a]
        length = lengths[(a, b)]
        planar = off[:, 0] ** 2 + off[:, 2] ** 2
        room = np.maximum(length**2 - planar, 0.0)
        sign = np.where(off[:, 1] < 0.0, -1.0, 1.0)
        new = off.copy()
        new[:, 1] = sign * np.sqrt(room)
        too_long = planar > length**2
        if np.any(too_long):
            k = length / np.sqrt(planar[too_long])
            new[too_long, 0] *= k
            new[too_long, 2] *= k
            new[too_long, 1] = 0.0
        pts[:, b] = pts[:, a] + new
    return pts


def _runs(mask):
    """(start, end exclusive) of every run of True in a 1-D bool array."""
    m = np.concatenate(([False], np.asarray(mask, bool), [False]))
    d = np.diff(m.astype(np.int8))
    return list(zip(np.flatnonzero(d == 1), np.flatnonzero(d == -1), strict=True))


def clean_runs(mask, min_run=3, max_gap=2):
    """Fill gaps of at most `max_gap` frames between runs, then drop runs under `min_run`."""
    m = np.asarray(mask, bool).copy()
    runs = _runs(m)
    for (_s0, e0), (s1, _e1) in zip(runs, runs[1:], strict=False):
        if s1 - e0 <= max_gap:
            m[e0:s1] = True
    for s, e in _runs(m):
        if e - s < min_run:
            m[s:e] = False
    return m


def foot_contacts(image, rate, scale=None, speed_max=0.35, lift_max=0.04, min_run=3, max_gap=2):
    """Planted feet (frames, 2) from normalised image landmarks (frames, 33, >=2).

    A foot is planted while its lowest point (heel or toe) is within `lift_max` body heights
    of the other foot's in the image and its speed is under `speed_max` body heights per
    second. `scale` (frames,) is the body height in image units (shoulders to ankles by
    default), which makes the thresholds independent of the distance to the camera.
    """
    img = np.asarray(image, dtype=np.float64)[..., :2]
    if scale is None:
        shoulders = 0.5 * (img[:, SHOULDER_L] + img[:, SHOULDER_R])
        ankles = 0.5 * (img[:, ANKLE_L] + img[:, ANKLE_R])
        scale = np.linalg.norm(ankles - shoulders, axis=-1)
    scale = np.maximum(np.asarray(scale, dtype=np.float64), 1e-6)
    low = []
    pos = []
    for ankle, heel, toe in FEET:
        low.append(np.maximum(img[:, heel, 1], img[:, toe, 1]))  # image y points down
        pos.append((img[:, ankle] + img[:, heel] + img[:, toe]) / 3.0)
    out = np.zeros((len(img), 2), bool)
    for f in range(2):
        v = np.zeros(len(img))
        if len(img) > 1:
            step = np.linalg.norm(np.diff(pos[f], axis=0), axis=-1) * rate
            v[1:] = step
            v[0] = step[0]
            v[1:-1] = np.minimum(step[:-1], step[1:])  # a planted frame is slow on one side
        lifted = (low[1 - f] - low[f]) / scale > lift_max
        out[:, f] = clean_runs((v / scale < speed_max) & ~lifted, min_run, max_gap)
    return out


def root_trajectory(points, contacts, damping=0.8):
    """Root (hip centre) positions (frames, 3) for hip-centred Z-up `points`.

    Horizontal: while feet stay planted the root moves by minus their mean motion relative to
    the hips; with no foot planted it keeps a damped velocity. Vertical: the lowest foot point
    (ankle, heel, toe) rests on the ground (Z = 0).
    """
    pts = np.asarray(points, dtype=np.float64)
    n = len(pts)
    root = np.zeros((n, 3))
    feet = np.stack([pts[:, list(f)].mean(axis=1) for f in FEET], axis=1)  # (n, 2, 3)
    vel = np.zeros(2)
    for t in range(1, n):
        both = contacts[t] & contacts[t - 1]
        if np.any(both):
            d = feet[t, both, :2] - feet[t - 1, both, :2]
            vel = -d.mean(axis=0)
        else:
            vel = vel * damping
        root[t, :2] = root[t - 1, :2] + vel
    lowest = pts[:, [i for f in FEET for i in f], 2].min(axis=1)
    root[:, 2] = -lowest
    return root


def resample(x, src_rate, dst_rate):
    """Linear resampling of `x` (frames first) from `src_rate` to `dst_rate` Hz."""
    x = np.asarray(x, dtype=np.float64)
    n = len(x)
    if n < 2 or src_rate == dst_rate:
        return x.copy()
    duration = (n - 1) / src_rate
    m = int(np.floor(duration * dst_rate + 1e-9)) + 1
    t = np.arange(m) * (src_rate / dst_rate)
    i0 = np.minimum(np.floor(t).astype(int), n - 2)
    w = (t - i0).reshape((-1,) + (1,) * (x.ndim - 1))
    return x[i0] * (1.0 - w) + x[i0 + 1] * w


def resample_mask(mask, src_rate, dst_rate):
    """Nearest-frame resampling of a bool mask (frames first)."""
    mask = np.asarray(mask, bool)
    n = len(mask)
    if n < 2 or src_rate == dst_rate:
        return mask.copy()
    m = int(np.floor((n - 1) / src_rate * dst_rate + 1e-9)) + 1
    idx = np.minimum(np.round(np.arange(m) * (src_rate / dst_rate)).astype(int), n - 1)
    return mask[idx]


def pin_targets(foot, contact, ground=None, ramp=2):
    """Locked positions and blend weights of one foot (frames, 3) for its planted runs.

    Each run of `contact` pins the foot at its mean horizontal position over the run and at
    its lowest height over the run (not below `ground` when given); the weight is 1 over the
    run and ramps down over `ramp` frames on each side, the target holding the run's value.
    Returns ``(targets (frames, 3), weights (frames,))``.
    """
    foot = np.asarray(foot, dtype=np.float64)
    n = len(foot)
    targets = foot.copy()
    weights = np.zeros(n)
    for s, e in _runs(contact):
        pin = foot[s:e].mean(axis=0)
        pin[2] = foot[s:e, 2].min()
        if ground is not None:
            pin[2] = max(pin[2], ground)
        for t in range(max(0, s - ramp), min(n, e + ramp)):
            if s <= t < e:
                w = 1.0
            elif t < s:
                w = 1.0 - (s - t) / (ramp + 1)
            else:
                w = 1.0 - (t - e + 1) / (ramp + 1)
            if w > weights[t]:
                weights[t] = w
                targets[t] = pin
    return targets, weights


def two_bone_ik(a, b, c, target, eps=1e-9):
    """New middle and end joints of chain `a`-`b`-`c` so that the end reaches `target`.

    Segment lengths are kept; the bend stays in the plane of the current chain (its pole is
    the current middle joint). An unreachable target is approached along the line from `a`.
    Returns ``(b_new, c_new)``.
    """
    a, b, c, target = (np.asarray(v, dtype=np.float64) for v in (a, b, c, target))
    l1 = np.linalg.norm(b - a)
    l2 = np.linalg.norm(c - b)
    to = target - a
    dist = np.linalg.norm(to)
    if dist < eps:
        return b.copy(), c.copy()
    axis = to / dist
    dist = float(np.clip(dist, abs(l1 - l2) + 1e-6, l1 + l2 - 1e-6))
    # Pole: component of the current middle joint off the a->target line.
    pole = (b - a) - axis * np.dot(b - a, axis)
    if np.linalg.norm(pole) < eps:
        pole = np.cross(axis, [1.0, 0.0, 0.0])
        if np.linalg.norm(pole) < eps:
            pole = np.cross(axis, [0.0, 1.0, 0.0])
    pole /= np.linalg.norm(pole)
    along = (l1**2 - l2**2 + dist**2) / (2.0 * dist)
    height = np.sqrt(max(l1**2 - along**2, 0.0))
    b_new = a + axis * along + pole * height
    c_new = a + axis * dist
    return b_new, c_new


def clean(world, image, rate, out_rate=24.0, min_cutoff=1.5, beta=0.3):
    """Full clean-up of one video clip.

    Returns a dict: ``points`` (frames, 33, 3) Z-up metres with the root motion added (at
    `out_rate`), ``root`` (frames, 3), ``contacts`` (frames, 2) bool, ``lengths`` (segment ->
    metres).
    """
    pts = to_zup(world)
    pts = one_euro(pts, rate, min_cutoff, beta)
    lengths = segment_lengths(pts)
    pts = enforce_lengths(pts, lengths=lengths)
    img = one_euro(np.asarray(image, dtype=np.float64)[..., :2], rate, min_cutoff, beta)
    contacts = foot_contacts(img, rate)
    root = one_euro(root_trajectory(pts, contacts), rate, 1.0, 0.0)
    pts = pts + root[:, None, :]
    return {
        "points": resample(pts, rate, out_rate),
        "root": resample(root, rate, out_rate),
        "contacts": resample_mask(contacts, rate, out_rate),
        "lengths": lengths,
    }
