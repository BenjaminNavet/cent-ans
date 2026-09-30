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


def foot_contacts(
    image,
    rate,
    scale=None,
    speed_max=0.35,
    lift_max=0.04,
    min_run=3,
    max_gap=2,
    heights=None,
    lift_m=0.07,
):
    """Planted feet (frames, 2) from normalised image landmarks (frames, 33, >=2).

    A foot is planted while its speed in the image is under `speed_max` body heights per
    second and it is not lifted: with `heights` (frames, 2), the 3D height of each foot's
    lowest point, not more than `lift_m` above the other foot; without, its lowest point
    within `lift_max` body heights of the other foot's in the image (fooled by a staggered
    stance: the rear foot stands higher in the image). `scale` (frames,) is the body height in
    image units (shoulders to ankles by default), which makes the thresholds independent of
    the distance to the camera.
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
            v[1:-1] = np.minimum(
                step[:-1], step[1:]
            )  # a planted frame is slow on one side
        if heights is not None:
            lifted = heights[:, f] - heights[:, 1 - f] > lift_m
        else:
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


def step_targets(foot, contact, ground=None, unit=1.0, max_gap=12, ramp=2):
    """Like ``pin_targets``, plus steps: a foot leaving one planted run for another one.

    NT14: between two planted runs at most `max_gap` frames apart, the foot is carried from the
    first pin to the second (smoothstep in the horizontal plane) on an arc lifted by a third of
    the step's length, 4 to 8 cm (`unit` = coordinate units per metre); the weight stays 1, so
    the video's shuffle (the foot skating along the ground) becomes a clean step. A step under
    3 cm keeps the foot down (the pins are merged into one).
    """
    foot = np.asarray(foot, dtype=np.float64)
    targets, weights = pin_targets(foot, contact, ground, ramp)
    runs = _runs(contact)
    for (_s0, e0), (s1, _e1) in zip(runs, runs[1:], strict=False):
        if s1 - e0 > max_gap:
            continue
        a = targets[e0 - 1].copy()
        b = targets[s1].copy()
        d = float(np.linalg.norm((b - a)[:2]))
        lift = (
            0.0
            if d < 0.03 * unit
            else float(np.clip(d / 3.0, 0.04 * unit, 0.08 * unit))
        )
        span = s1 - e0 + 1
        for t in range(e0, s1):
            u = (t - e0 + 1) / span
            k = u * u * (3.0 - 2.0 * u)
            p = a * (1.0 - k) + b * k
            p[2] = a[2] * (1.0 - u) + b[2] * u + lift * np.sin(np.pi * u)
            targets[t] = p
            weights[t] = 1.0
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
    # Pole: side of the current bend (middle joint off the current a->c line), made
    # perpendicular to the new a->target line.
    cur = c - a
    cur_len = np.linalg.norm(cur)
    pole = b - a
    if cur_len > eps:
        pole = pole - cur / cur_len * np.dot(pole, cur / cur_len)
    pole = pole - axis * np.dot(pole, axis)
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


def ground_tilt(points, above=0.03, rounds=3):
    """Rotation (3, 3) levelling the ground under Z-up `points` (frames, 33, 3).

    MediaPipe's world frame follows the camera: a camera looking down tilts the ground, and a
    foot farther from the camera then seems higher (NT13-NT14: a staggered stance read as a
    lifted foot, no contact, the foot sliding). The ground line ``Z = b Y + c`` is fitted to
    the heel and toe points, dropping those more than `above` metres over it (lifted feet) for
    `rounds` rounds; the rotation turns its normal onto +Z (tilts over 25 degrees are ignored:
    not a camera pitch). Pitch only: a phone on a tripod does not roll, and a roll fitted to
    noisy feet leans the whole body sideways.
    """
    pts = np.asarray(points, dtype=np.float64)
    idx = [HEEL_L, TOE_L, HEEL_R, TOE_R]
    p = pts[:, idx].reshape(-1, 3)
    keep = np.ones(len(p), bool)
    coef = np.zeros(2)
    for _ in range(rounds):
        a = np.stack((p[keep, 1], np.ones(keep.sum())), axis=1)
        coef, *_ = np.linalg.lstsq(a, p[keep, 2], rcond=None)
        resid = p[:, 2] - (coef[0] * p[:, 1] + coef[1])
        keep = resid < above
        if keep.sum() < 8:
            break
    n = np.array([0.0, -coef[0], 1.0])
    n /= np.linalg.norm(n)
    angle = np.arccos(np.clip(n[2], -1.0, 1.0))
    if angle < 1e-6 or angle > np.radians(25.0):
        return np.eye(3)
    axis = np.cross(n, [0.0, 0.0, 1.0])
    axis /= np.linalg.norm(axis)
    k = np.array(
        [[0.0, -axis[2], axis[1]], [axis[2], 0.0, -axis[0]], [-axis[1], axis[0], 0.0]]
    )
    return np.eye(3) + np.sin(angle) * k + (1.0 - np.cos(angle)) * (k @ k)


def run_frames(seconds, rate, least=1):
    """A duration in seconds as a number of frames at `rate` (at least `least`)."""
    return max(least, int(round(seconds * rate)))


def clean(
    world,
    image,
    rate,
    out_rate=24.0,
    min_cutoff=1.5,
    beta=0.3,
    extra=None,
    level_ground=True,
):
    """Full clean-up of one video clip.

    `rate` is the playback rate of the source frames (video rate times the speed-up). NT14:
    contact runs are durations (0.1 s runs, 0.067 s gaps: 3 and 2 frames at 30 fps, twice as
    many at 60 fps); `extra` (name -> (frames, 3) hip-centred Z-up points, e.g. the shield disc
    of ``disc_points``) gets the same levelling, root motion and resampling; `level_ground`
    rotates everything so that the ground under the heels and toes is flat (``ground_tilt``).

    Returns a dict: ``points`` (frames, 33, 3) Z-up metres with the root motion added (at
    `out_rate`), ``root`` (frames, 3), ``contacts`` (frames, 2) bool, ``lengths`` (segment ->
    metres), ``level`` (the levelling rotation), ``extra`` (name -> resampled points).
    """
    pts = to_zup(world)
    pts = one_euro(pts, rate, min_cutoff, beta)
    lengths = segment_lengths(pts)
    pts = enforce_lengths(pts, lengths=lengths)
    level = ground_tilt(pts) if level_ground else np.eye(3)
    pts = pts @ level.T
    extra = {
        k: np.asarray(v, dtype=np.float64) @ level.T for k, v in (extra or {}).items()
    }
    img = one_euro(np.asarray(image, dtype=np.float64)[..., :2], rate, min_cutoff, beta)
    heights = np.stack(
        [pts[:, [heel, toe], 2].min(axis=1) for _a, heel, toe in FEET], 1
    )
    contacts = foot_contacts(
        img,
        rate,
        heights=heights,
        min_run=run_frames(0.1, rate, 3),
        max_gap=run_frames(0.067, rate, 2),
    )
    root = one_euro(root_trajectory(pts, contacts), rate, 1.0, 0.0)
    pts = pts + root[:, None, :]
    return {
        "points": resample(pts, rate, out_rate),
        "root": resample(root, rate, out_rate),
        "contacts": resample_mask(contacts, rate, out_rate),
        "lengths": lengths,
        "level": level,
        "extra": {
            k: resample(np.asarray(v, dtype=np.float64) + root, rate, out_rate)
            for k, v in (extra or {}).items()
        },
    }


# --- NT14: shield disc, wrist spikes, facing ---------------------------------------------

# Landmarks trusted to map the image onto the world frame (the left arm hides behind the disc).
FIT_LANDMARKS = (
    SHOULDER_L,
    SHOULDER_R,
    ELBOW_R,
    WRIST_R,
    HIP_L,
    HIP_R,
    KNEE_L,
    KNEE_R,
    ANKLE_L,
    ANKLE_R,
)


def image_world_fit(world, image, aspect, visibility=None, vis_min=0.7):
    """Per-frame scale and offsets mapping image points onto the Z-up world frame.

    World X ~ ``s * x * aspect + tx`` and world Z ~ ``-s * y + tz`` (image x right, y down,
    normalised by the width and the height; `aspect` = width / height), least squares over the
    `FIT_LANDMARKS` seen with a visibility of at least `vis_min`. Returns ``(s, tx, tz)``, each
    (frames,); `s` is in metres per image height.
    """
    w = to_zup(world)
    img = np.asarray(image, dtype=np.float64)
    n = len(w)
    s, tx, tz = np.zeros(n), np.zeros(n), np.zeros(n)
    idx = list(FIT_LANDMARKS)
    for t in range(n):
        keep = idx
        if visibility is not None:
            good = [i for i in idx if visibility[t, i] >= vis_min]
            keep = good if len(good) >= 4 else idx
        k = len(keep)
        a = np.zeros((2 * k, 3))
        a[:k, 0] = img[t, keep, 0] * aspect
        a[k:, 0] = -img[t, keep, 1]
        a[:k, 1] = 1.0
        a[k:, 2] = 1.0
        v = np.concatenate((w[t, keep, 0], w[t, keep, 2]))
        sol, *_ = np.linalg.lstsq(a, v, rcond=None)
        s[t], tx[t], tz[t] = sol
    return s, tx, tz


def disc_points(centre, major, fit, aspect, anchor_depth=0.0, fov_deg=65.0):
    """Hip-centred Z-up positions (frames, 3) of a tracked disc's centre.

    X and Z come from the image centre through `fit` (``image_world_fit``). Depth: the camera
    distance of the body is ``s / (2 tan(fov / 2))`` (`fov_deg` = field of view along the
    image height); the disc's apparent size against its median gives its distance relative to
    the body's, `anchor_depth` being the depth (Y, metres) of the disc at its median size.
    Perspective: a disc nearer than the body (the hips, at the body's distance) is seen farther
    from the image centre than it is, its offset from the optical axis is scaled back by the
    ratio of the distances.
    """
    s, tx, tz = (np.asarray(v, dtype=np.float64) for v in fit)
    c = np.asarray(centre, dtype=np.float64)
    size = np.asarray(major, dtype=np.float64) * s  # metres at the body's distance
    dist = s / (2.0 * np.tan(np.radians(fov_deg) / 2.0))
    ref = np.median(size)
    y = anchor_depth + dist * (ref / np.maximum(size, 1e-6) - 1.0)
    k = np.maximum(dist + y, 0.2) / dist
    axis_x = s * 0.5 * aspect + tx
    axis_z = -s * 0.5 + tz
    x = axis_x + (s * c[:, 0] * aspect + tx - axis_x) * k
    z = axis_z + (-s * c[:, 1] + tz - axis_z) * k
    return np.stack((x, y, z), axis=-1)


def reach_anchor(points_at, shoulder, reach, lo=-1.0, hi=0.3, steps=261):
    """Depth anchor (Y, metres) putting a hand-held disc at most `reach` from the shoulder.

    `points_at(anchor)` gives the disc (frames, 3) for an anchor depth (``disc_points``): the
    size gives its depth only up to that offset. The anchor kept is the one in front of the
    body (towards the camera, -Y) where the farthest frame is exactly `reach` from `shoulder`
    (frames, 3): the disc is in the hand, and the widest block or push stretches the arm.
    """
    sh = np.asarray(shoulder, dtype=np.float64)
    anchors = np.linspace(hi, lo, steps)
    far = np.array(
        [np.max(np.linalg.norm(points_at(a) - sh, axis=-1)) for a in anchors]
    )
    k = int(np.argmin(far))
    if far[k] >= reach:
        return float(anchors[k])
    beyond = np.flatnonzero(far[k:] >= reach)
    return float(anchors[k + beyond[0]]) if len(beyond) else float(lo)


def disc_normals(major, minor, angle, disc, chest, keep=0.7):
    """Unit normals (frames, 3) of the disc's front face (Z-up, Y away from the camera).

    The ellipse's axis ratio gives the tilt out of the image plane, its minor axis the tilt's
    direction; of the four candidates (tilt sign, face sign) the one facing most away from
    `chest` (frames, 3) towards the disc is kept (a shield's face looks out), plus `keep`
    times its agreement with the previous frame's pick (the tilt sign is ambiguous frame by
    frame: without it the pick flips back and forth when the disc faces the camera).
    """
    ratio = np.clip(np.asarray(minor) / np.maximum(np.asarray(major), 1e-9), 0.0, 1.0)
    phi = np.arccos(ratio)
    ang = np.asarray(angle, dtype=np.float64) + np.pi / 2.0  # minor axis in the image
    m = np.stack(
        (np.cos(ang), np.zeros_like(ang), -np.sin(ang)), axis=-1
    )  # y down -> -Z
    toward = np.array([0.0, -1.0, 0.0])
    out = np.asarray(disc, dtype=np.float64) - np.asarray(chest, dtype=np.float64)
    out /= np.maximum(np.linalg.norm(out, axis=-1, keepdims=True), 1e-9)
    res = np.zeros_like(out)
    for t in range(len(phi)):
        best, score = None, -np.inf
        for tilt in (1.0, -1.0):
            n = np.cos(phi[t]) * toward + tilt * np.sin(phi[t]) * m[t]
            for face in (1.0, -1.0):
                sc = float(np.dot(face * n, out[t]))
                if t > 0:
                    sc += keep * float(np.dot(face * n, res[t - 1]))
                if sc > score:
                    best, score = face * n, sc
        res[t] = best
    return res


def smooth_directions(v, rate, min_cutoff=1.5, beta=0.3):
    """Zero-phase One-Euro smoothing of unit vectors (frames, 3), renormalised."""
    out = one_euro(np.asarray(v, dtype=np.float64), rate, min_cutoff, beta)
    return out / np.maximum(np.linalg.norm(out, axis=-1, keepdims=True), 1e-9)


def quat_angle(a, b):
    """Angle (radians) between unit quaternions (w, x, y, z), last axis."""
    d = np.abs(np.sum(np.asarray(a) * np.asarray(b), axis=-1))
    return 2.0 * np.arccos(np.clip(d, 0.0, 1.0))


def continuous_quats(q):
    """Unit quaternions (frames, 4) with signs flipped to stay on one hemisphere."""
    q = np.array(q, dtype=np.float64)
    for t in range(1, len(q)):
        if np.dot(q[t], q[t - 1]) < 0.0:
            q[t] = -q[t]
    return q


def despike_quats(q, spike_deg=12.0):
    """Unit quaternions (frames, 4) with isolated spikes replaced by their neighbours' mean.

    A frame is a spike when it is more than `spike_deg` away from both neighbours while the
    neighbours are close to each other (less than half of that): a wrist cannot flick out and
    back within one frame.
    """
    q = continuous_quats(q)
    lim = np.radians(spike_deg)
    for t in range(1, len(q) - 1):
        a = quat_angle(q[t - 1], q[t])
        b = quat_angle(q[t], q[t + 1])
        c = quat_angle(q[t - 1], q[t + 1])
        if min(a, b) > lim and c < 0.5 * min(a, b):
            m = q[t - 1] + q[t + 1]
            q[t] = m / np.linalg.norm(m)
    return q


def smooth_quats(q, rate, min_cutoff=2.0, beta=0.2):
    """Zero-phase One-Euro smoothing of unit quaternions (frames, 4), sign-continuous."""
    out = one_euro(continuous_quats(q), rate, min_cutoff, beta)
    return out / np.maximum(np.linalg.norm(out, axis=-1, keepdims=True), 1e-9)


def facing_yaw(points, reach=WRIST_R, top=0.2):
    """Direction of the opponent (degrees about up; 0 = towards the camera, 90 = image right).

    Mean of the chest's facing (normal of the shoulder line, horizontal) and of the reaching
    hand's direction from the hips, over the `top` fraction of frames where that hand is
    farthest from the hips horizontally (the strike or the block): the adversary is where the
    blow lands and where the chest turns to. Z-up points (frames, 33, 3), X image right (the
    performer's left when facing the camera), Y away from the camera. `reach` is a landmark
    index or a (frames, 3) track (the shield disc).
    """
    pts = np.asarray(points, dtype=np.float64)
    hips = 0.5 * (pts[:, HIP_L] + pts[:, HIP_R])
    if np.ndim(reach) == 0:
        reach = pts[:, reach]
    hand = np.asarray(reach, dtype=np.float64) - hips
    hand[:, 2] = 0.0
    dist = np.linalg.norm(hand, axis=-1)
    k = max(1, int(round(top * len(pts))))
    sel = np.argsort(dist)[-k:]
    left = pts[sel, SHOULDER_L] - pts[sel, SHOULDER_R]
    face = np.cross(left, [0.0, 0.0, 1.0])  # left x up = forward
    face[:, 2] = 0.0
    face /= np.maximum(np.linalg.norm(face, axis=-1, keepdims=True), 1e-9)
    hand_dir = hand[sel] / np.maximum(dist[sel, None], 1e-9)
    d = face.mean(axis=0) + hand_dir.mean(axis=0)
    return float(np.degrees(np.arctan2(d[0], -d[1])))  # forward = (sin, -cos)
