# /// script
# requires-python = ">=3.10,<3.13"
# dependencies = ["opencv-python-headless>=4.9", "numpy>=1.26"]
# ///
"""Lot AS8a: image shift of a handheld camera, to keep the feet's image speed honest.

The contacts of ``video_mocap_clean.foot_contacts`` read the speed of each foot in the image;
a panning camera makes a planted foot look fast, so no foot is "planted" and the figure slides.
This tool tracks background corners (the pose's bounding box is masked out) frame to frame
with Lucas-Kanade flow, takes their median translation and accumulates it. The clip loader
subtracts the result from the normalised image landmarks.
Same privacy rule as ``extract_pose.py``: videos and outputs stay outside the repository::

    uv run tools/video_mocap/camera_shift.py VIDEO POSES.npz OUT.npz

Output ``.npz``: ``shift`` (frames, 2) cumulative background translation in normalised image
x, y (x by the width, y by the height), 0 at the first frame.
"""

import argparse
import sys

import cv2
import numpy as np


def person_mask(shape, image_points, margin=0.08):
    """Mask (255 = usable background) hiding the box around the person's landmarks."""
    h, w = shape[:2]
    mask = np.full((h, w), 255, np.uint8)
    xs, ys = image_points[:, 0], image_points[:, 1]
    x0, x1 = (xs.min() - margin) * w, (xs.max() + margin) * w
    y0, y1 = (ys.min() - margin) * h, (ys.max() + margin) * h
    mask[max(int(y0), 0) : int(y1) + 1, max(int(x0), 0) : int(x1) + 1] = 0
    return mask


def step_shift(prev, cur, mask):
    """Median translation (pixels) of the background between two gray frames, or None."""
    pts = cv2.goodFeaturesToTrack(prev, 300, 0.01, 8, mask=mask)
    if pts is None or len(pts) < 8:
        return None
    nxt, ok, _err = cv2.calcOpticalFlowPyrLK(prev, cur, pts, None)
    good = ok.ravel() == 1
    if good.sum() < 8:
        return None
    return np.median((nxt - pts)[good].reshape(-1, 2), axis=0)


def main() -> int:
    """Track the camera shift of every frame of a video."""
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("video")
    ap.add_argument("poses")
    ap.add_argument("out")
    ap.add_argument("--width", type=int, default=640)
    args = ap.parse_args()
    image = np.load(args.poses)["image"][..., :2]
    cap = cv2.VideoCapture(args.video)
    if not cap.isOpened():
        print("cannot open", args.video, file=sys.stderr)
        return 1
    shifts = [np.zeros(2)]
    prev = None
    index = 0
    while True:
        ok, frame = cap.read()
        if not ok or index >= len(image):
            break
        scale = args.width / frame.shape[1]
        gray = cv2.cvtColor(
            cv2.resize(frame, None, fx=scale, fy=scale, interpolation=cv2.INTER_AREA),
            cv2.COLOR_BGR2GRAY,
        )
        if prev is not None:
            mask = person_mask(gray.shape, image[index])
            d = step_shift(prev, gray, mask)
            d = (
                np.zeros(2)
                if d is None
                else d / np.array([gray.shape[1], gray.shape[0]])
            )
            shifts.append(shifts[-1] + d)
        prev = gray
        index += 1
    np.savez(args.out, shift=np.array(shifts))
    print("OK", args.out, "frames", len(shifts), "total", shifts[-1])
    return 0


if __name__ == "__main__":
    sys.exit(main())
