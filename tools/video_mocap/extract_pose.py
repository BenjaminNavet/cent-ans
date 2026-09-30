# /// script
# requires-python = ">=3.10,<3.13"
# dependencies = ["mediapipe>=0.10.14", "opencv-python-headless>=4.9", "numpy>=1.26"]
# ///
"""Lot NT13: 3D pose landmarks of a phone video (MediaPipe Pose Landmarker "heavy").

Videos are the player's own and personal: they, and everything extracted from them, stay
outside the (public) repository, in ``~/dev/cent-ans-mocap-src/work/``::

    uv run tools/video_mocap/extract_pose.py VIDEO.MOV MODEL.task OUT.npz [--long-side 1920]

``MODEL.task``: ``pose_landmarker_heavy.task`` (Google MediaPipe model card, Apache 2.0),
https://storage.googleapis.com/mediapipe-models/pose_landmarker/pose_landmarker_heavy/float16/latest/pose_landmarker_heavy.task

Output ``.npz``: ``world`` (frames, 33, 3) metres, hip-centred, MediaPipe axes (x left of the
image, y down, z towards the camera negative); ``image`` (frames, 33, 3) normalised image x, y
and relative depth; ``visibility`` and ``presence`` (frames, 33); ``fps``; ``size`` (width,
height after orientation); ``found`` (frames,) bool. Frames without a detection repeat the
previous pose (``found`` false).

Licences: MediaPipe (Apache 2.0), pose_landmarker_heavy model (Apache 2.0), OpenCV (Apache
2.0), NumPy (BSD-3). No SMPL / SMPL-X / AMASS / HumanML3D dependency.
"""

import argparse
import sys
import time

import cv2
import mediapipe as mp
import numpy as np
from mediapipe.tasks.python import BaseOptions, vision


def main() -> int:
    """Extract the landmarks of every frame of a video."""
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("video")
    ap.add_argument("model")
    ap.add_argument("out")
    ap.add_argument("--long-side", type=int, default=1920)
    args = ap.parse_args()

    cap = cv2.VideoCapture(args.video)
    if not cap.isOpened():
        print("cannot open", args.video, file=sys.stderr)
        return 1
    fps = cap.get(cv2.CAP_PROP_FPS) or 30.0
    opts = vision.PoseLandmarkerOptions(
        base_options=BaseOptions(model_asset_path=args.model),
        running_mode=vision.RunningMode.VIDEO,
        num_poses=1,
        min_pose_detection_confidence=0.5,
        min_pose_presence_confidence=0.5,
        min_tracking_confidence=0.5,
    )
    world, image, vis, pres, found = [], [], [], [], []
    size = (0, 0)
    t0 = time.time()
    with vision.PoseLandmarker.create_from_options(opts) as marker:
        index = 0
        while True:
            ok, frame = cap.read()
            if not ok:
                break
            h, w = frame.shape[:2]
            scale = args.long_side / max(h, w)
            if scale < 1.0:
                frame = cv2.resize(frame, (round(w * scale), round(h * scale)), interpolation=cv2.INTER_AREA)
            size = (frame.shape[1], frame.shape[0])
            rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
            res = marker.detect_for_video(
                mp.Image(image_format=mp.ImageFormat.SRGB, data=rgb),
                int(round(index * 1000.0 / fps)),
            )
            if res.pose_world_landmarks:
                wl = res.pose_world_landmarks[0]
                il = res.pose_landmarks[0]
                world.append([(p.x, p.y, p.z) for p in wl])
                image.append([(p.x, p.y, p.z) for p in il])
                vis.append([p.visibility for p in wl])
                pres.append([p.presence for p in wl])
                found.append(True)
            elif world:
                world.append(world[-1])
                image.append(image[-1])
                vis.append([0.0] * 33)
                pres.append([0.0] * 33)
                found.append(False)
            index += 1
    if not world:
        print("no pose found", file=sys.stderr)
        return 1
    np.savez_compressed(
        args.out,
        world=np.asarray(world, np.float32),
        image=np.asarray(image, np.float32),
        visibility=np.asarray(vis, np.float32),
        presence=np.asarray(pres, np.float32),
        found=np.asarray(found, bool),
        fps=np.float32(fps),
        size=np.asarray(size, np.int32),
    )
    print(f"OK {args.out} frames={len(world)} found={sum(found)} size={size} fps={fps:.2f} time={time.time() - t0:.1f}s")
    return 0


if __name__ == "__main__":
    sys.exit(main())
