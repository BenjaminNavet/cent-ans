"""Lot NT13: clean-up of video pose landmarks (smoothing, depth, feet, root, IK)."""

import sys
from pathlib import Path

import numpy as np
import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "blender_scripts"))

import video_mocap_clean as vc  # noqa: E402


def test_to_zup_is_a_proper_rotation():
    """MediaPipe y-down / z-away becomes Z-up / Y-away, right handed."""
    pts = np.array([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
    out = vc.to_zup(pts)
    assert np.allclose(out[0], [1, 0, 0])
    assert np.allclose(out[1], [0, 0, -1])  # down in the image is -Z
    assert np.allclose(out[2], [0, 1, 0])
    assert np.isclose(np.linalg.det(out), 1.0)


def test_one_euro_removes_jitter_without_lag():
    """Jitter drops sharply; a slow ramp is followed with no lag (zero phase)."""
    rng = np.random.default_rng(3)
    t = np.arange(120) / 30.0
    clean = 0.3 * t
    noisy = clean + rng.normal(0.0, 0.01, t.shape)
    out = vc.one_euro(noisy, 30.0, min_cutoff=1.0, beta=0.0)
    inner = slice(10, -10)
    assert np.std(out[inner] - clean[inner]) < 0.5 * np.std(noisy[inner] - clean[inner])
    assert abs(np.mean(out[inner] - clean[inner])) < 0.003


def test_one_euro_keeps_fast_motion_with_beta():
    """A fast strike keeps more of its amplitude with a speed coefficient."""
    t = np.arange(60) / 30.0
    strike = np.exp(-(((t - 1.0) / 0.06) ** 2))
    slow = vc.one_euro(strike, 30.0, min_cutoff=1.0, beta=0.0)
    fast = vc.one_euro(strike, 30.0, min_cutoff=1.0, beta=2.0)
    assert fast.max() > slow.max()
    assert fast.max() > 0.6


def test_enforce_lengths_restores_depth():
    """A foreshortened segment regains its median length through its depth only."""
    n = 11
    pts = np.zeros((n, 33, 3))
    pts[:, vc.SHOULDER_R] = [0.2, 0.0, 1.4]
    arm = np.tile([0.0, 0.0, -0.3], (n, 1))
    arm[5] = [0.0, -0.1, -0.2]  # frame 5: too short, pointing towards the camera
    pts[:, vc.ELBOW_R] = pts[:, vc.SHOULDER_R] + arm
    out = vc.enforce_lengths(pts, segments=((vc.SHOULDER_R, vc.ELBOW_R),))
    seg = out[:, vc.ELBOW_R] - out[:, vc.SHOULDER_R]
    assert np.allclose(np.linalg.norm(seg, axis=-1), 0.3)
    assert (
        np.isclose(seg[5, 2], -0.2) and seg[5, 1] < 0.0
    )  # in-image offset and sign kept


def test_enforce_lengths_moves_the_subtree():
    """Correcting an elbow carries the wrist along."""
    pts = np.zeros((3, 33, 3))
    pts[:, vc.ELBOW_L] = [0.0, 0.0, -0.1]
    pts[:, vc.WRIST_L] = [0.0, 0.0, -0.35]
    pts[1, vc.ELBOW_L] = [0.0, 0.0, -0.3]  # stretched upper arm on frame 1
    pts[1, vc.WRIST_L] = [0.0, 0.0, -0.55]
    segs = ((vc.SHOULDER_L, vc.ELBOW_L), (vc.ELBOW_L, vc.WRIST_L))
    out = vc.enforce_lengths(pts, segments=segs)
    assert np.allclose(np.linalg.norm(out[1, vc.ELBOW_L] - out[1, vc.SHOULDER_L]), 0.1)
    assert np.allclose(out[1, vc.WRIST_L] - out[1, vc.ELBOW_L], [0.0, 0.0, -0.25])


def test_clean_runs():
    """Short gaps are filled, then short runs dropped."""
    m = np.array([1, 1, 0, 1, 1, 0, 0, 0, 1, 0, 0, 0, 1, 1, 1, 1], bool)
    out = vc.clean_runs(m, min_run=3, max_gap=1)
    assert out.tolist() == [1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1]


def _walking_image(n=40):
    """Image landmarks: left foot planted, right foot stepping (lifted mid-clip)."""
    img = np.zeros((n, 33, 3))
    img[:, vc.SHOULDER_L] = [0.55, 0.3, 0]
    img[:, vc.SHOULDER_R] = [0.45, 0.3, 0]
    for ankle, heel, toe in vc.FEET:
        img[:, ankle, 1] = 0.85
        img[:, heel, 1] = 0.9
        img[:, toe, 1] = 0.9
    for i in vc.FEET[0]:
        img[:, i, 0] = 0.55
    step = np.clip((np.arange(n) - 10) / 20.0, 0.0, 1.0)
    lift = np.sin(np.pi * step) * 0.06
    for i in vc.FEET[1]:
        img[:, i, 0] = 0.45 - 0.1 * step
        img[:, i, 1] -= lift
    return img


def test_foot_contacts_planted_and_stepping():
    """The still foot is planted throughout; the stepping one is off in mid-step only."""
    contacts = vc.foot_contacts(_walking_image(), 30.0)
    assert contacts[:, 0].all()
    assert contacts[:5, 1].all() and contacts[-5:, 1].all()
    assert not contacts[15:25, 1].any()


def test_foot_contacts_staggered_stance_uses_heights():
    """A rear foot higher in the image is still planted when its 3D height says so."""
    img = _walking_image()
    for i in vc.FEET[1]:
        img[:, i, 1] = (
            img[:, vc.FEET[0][0], 1] - 0.08
        )  # static, 8 % higher in the image
        img[:, i, 0] = 0.45
    assert not vc.foot_contacts(img, 30.0)[:, 1].any()
    heights = np.zeros((len(img), 2))
    assert vc.foot_contacts(img, 30.0, heights=heights)[:, 1].all()


def test_root_trajectory_follows_the_planted_foot():
    """A planted foot sliding back relative to the hips means the body moved forward."""
    n = 10
    pts = np.zeros((n, 33, 3))
    for i in vc.FEET[0] + vc.FEET[1]:
        pts[:, i, 2] = -0.9
    for i in vc.FEET[0]:
        pts[:, i, 1] = -0.02 * np.arange(n)  # relative to the hips, the foot goes back
    contacts = np.zeros((n, 2), bool)
    contacts[:, 0] = True
    root = vc.root_trajectory(pts, contacts)
    assert np.allclose(root[:, 1], 0.02 * np.arange(n))
    assert np.allclose(root[:, 2], 0.9)  # lowest foot on the ground


def test_resample_rate_and_ends():
    """30 -> 24 fps keeps the ends and the duration."""
    x = np.arange(31, dtype=float)  # one second at 30 fps
    out = vc.resample(x, 30.0, 24.0)
    assert len(out) == 25
    assert out[0] == 0.0 and np.isclose(out[-1], 30.0)
    mask = vc.resample_mask(np.arange(31) < 15, 30.0, 24.0)
    assert len(mask) == 25 and mask[0] and not mask[-1]


def test_pin_targets_hold_and_ramp():
    """A planted run is pinned at its mean position and lowest height, with ramps."""
    n = 12
    foot = np.zeros((n, 3))
    foot[:, 0] = np.linspace(0.0, 0.11, n)
    foot[:, 2] = 0.05
    foot[5, 2] = 0.02
    contact = np.zeros(n, bool)
    contact[4:8] = True
    targets, weights = vc.pin_targets(foot, contact, ground=0.03, ramp=2)
    assert weights[4:8].tolist() == [1.0] * 4
    assert 0.0 < weights[3] < 1.0 and 0.0 < weights[8] < 1.0
    assert weights[0] == 0.0 and weights[-1] == 0.0
    assert np.allclose(targets[4:8, 0], foot[4:8, 0].mean())
    assert np.allclose(targets[4:8, 2], 0.03)  # lowest height, not below the ground
    assert np.allclose(targets[3], targets[4])


@pytest.mark.parametrize(
    "target", [[0.1, 0.05, -0.7], [0.0, 0.3, -0.5], [0.0, 0.0, -2.0]]
)
def test_two_bone_ik_keeps_lengths(target):
    """Lengths kept, target reached when reachable, bend on the side of the old knee."""
    a = np.array([0.0, 0.0, 0.0])
    b = np.array([0.0, 0.1, -0.42])
    c = np.array([0.0, 0.0, -0.84])
    b2, c2 = vc.two_bone_ik(a, b, c, target)
    assert np.isclose(np.linalg.norm(b2 - a), np.linalg.norm(b - a))
    assert np.isclose(np.linalg.norm(c2 - b2), np.linalg.norm(c - b))
    target = np.asarray(target)
    reach = np.linalg.norm(b - a) + np.linalg.norm(c - b)
    if np.linalg.norm(target - a) < reach:
        assert np.allclose(c2, target, atol=1e-5)
        assert b2[1] > 0.0
    else:
        assert np.linalg.norm(c2 - a) == pytest.approx(reach, abs=1e-4)


def test_clean_end_to_end_shapes():
    """The full chain returns resampled points on the ground with contacts."""
    n = 30
    rng = np.random.default_rng(0)
    world = np.zeros((n, 33, 3))
    world[:, :, 1] = -0.5  # everything above the hips (y down)
    for i in vc.FEET[0] + vc.FEET[1]:
        world[:, i, 1] = 0.9
    world += rng.normal(0.0, 0.003, world.shape)
    out = vc.clean(world, _walking_image(n), 30.0)
    assert out["points"].shape == (24, 33, 3)
    assert out["contacts"].shape == (24, 2)
    feet = out["points"][:, [i for f in vc.FEET for i in f], 2]
    assert np.allclose(feet.min(axis=1), 0.0, atol=0.02)
