# /// script
# requires-python = ">=3.10,<3.13"
# dependencies = ["opencv-python-headless>=4.9", "numpy>=1.26"]
# ///
"""Lot NT14: track the stand-in shield (a saturated red disc held in the left hand) in a video.

MediaPipe loses the left forearm behind the disc (elbow and wrist visibility under 0.25), so
the shield arm is solved from the disc itself: its centre (where the fist is) and its ellipse
(the disc's tilt). Same privacy rule as ``extract_pose.py``: videos and outputs stay outside
the repository::

    uv run tools/video_mocap/track_disc.py VIDEO.MOV OUT.npz [--width 540]

Output ``.npz``: ``centre`` (frames, 2) normalised image x, y (after orientation);
``axes`` (frames, 2) full major and minor axis lengths in image heights; ``angle`` (frames,)
direction of the major axis in the image (radians, x right, y down); ``area`` (frames,) in
image heights squared; ``found`` (frames,) bool (a frame without a disc repeats the last one).

Detection: HSV threshold on saturated red (hue wraps around 0), morphological opening and
closing, the largest blob larger than ``--min-area``; ``cv2.fitEllipse`` on its outer contour.
"""

import argparse
import sys

import cv2
import numpy as np


def red_mask(bgr, sat_min=120, val_min=50, hue_band=10):
    """Binary mask of saturated red pixels."""
    hsv = cv2.cvtColor(bgr, cv2.COLOR_BGR2HSV)
    low = cv2.inRange(hsv, (0, sat_min, val_min), (hue_band, 255, 255))
    high = cv2.inRange(hsv, (180 - hue_band, sat_min, val_min), (180, 255, 255))
    mask = cv2.bitwise_or(low, high)
    kernel = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (5, 5))
    mask = cv2.morphologyEx(mask, cv2.MORPH_OPEN, kernel)
    return cv2.morphologyEx(mask, cv2.MORPH_CLOSE, kernel)


def disc_of(mask, min_area):
    """(centre x, y, major, minor, angle rad, area) in pixels of the largest blob, or None."""
    contours, _ = cv2.findContours(mask, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_NONE)
    if not contours:
        return None
    best = max(contours, key=cv2.contourArea)
    area = cv2.contourArea(best)
    if area < min_area or len(best) < 5:
        return None
    (cx, cy), (d1, d2), deg = cv2.fitEllipse(best)
    # OpenCV: `deg` rotates the first axis (d1) from the image x axis.
    angle = np.radians(deg)
    if d2 > d1:
        d1, d2 = d2, d1
        angle += np.pi / 2.0
    return cx, cy, d1, d2, float(np.arctan2(np.sin(angle), np.cos(angle))), area


def main() -> int:
    """Track the disc in every frame of a video."""
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("video")
    ap.add_argument("out")
    ap.add_argument("--width", type=int, default=540)
    ap.add_argument("--min-area", type=float, default=300.0)
    args = ap.parse_args()
    cap = cv2.VideoCapture(args.video)
    if not cap.isOpened():
        print("cannot open", args.video, file=sys.stderr)
        return 1
    cap.set(cv2.CAP_PROP_ORIENTATION_AUTO, 0)
    turn = {
        90: cv2.ROTATE_90_CLOCKWISE,
        -90: cv2.ROTATE_90_COUNTERCLOCKWISE,
        270: cv2.ROTATE_90_COUNTERCLOCKWISE,
        180: cv2.ROTATE_180,
    }.get(int(round(cap.get(cv2.CAP_PROP_ORIENTATION_META))))
    centre, axes, angle, area, found = [], [], [], [], []
    while True:
        ok, frame = cap.read()
        if not ok:
            break
        if turn is not None:
            frame = cv2.rotate(frame, turn)
        h, w = frame.shape[:2]
        k = args.width / w
        frame = cv2.resize(
            frame, (args.width, round(h * k)), interpolation=cv2.INTER_AREA
        )
        h, w = frame.shape[:2]
        disc = disc_of(red_mask(frame), args.min_area)
        if disc is None:
            if centre:
                centre.append(centre[-1])
                axes.append(axes[-1])
                angle.append(angle[-1])
                area.append(area[-1])
            else:
                centre.append((np.nan, np.nan))
                axes.append((np.nan, np.nan))
                angle.append(np.nan)
                area.append(np.nan)
            found.append(False)
            continue
        cx, cy, major, minor, ang, a = disc
        centre.append((cx / w, cy / h))
        axes.append((major / h, minor / h))
        angle.append(ang)
        area.append(a / (h * h))
        found.append(True)
    if not any(found):
        print("no disc found", file=sys.stderr)
        return 1
    np.savez_compressed(
        args.out,
        centre=np.asarray(centre, np.float32),
        axes=np.asarray(axes, np.float32),
        angle=np.asarray(angle, np.float32),
        area=np.asarray(area, np.float32),
        found=np.asarray(found, bool),
    )
    print(f"OK {args.out} frames={len(found)} found={sum(found)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
