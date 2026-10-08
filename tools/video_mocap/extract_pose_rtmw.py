# /// script
# requires-python = ">=3.10,<3.13"
# dependencies = ["rtmlib", "onnxruntime", "opencv-python-headless>=4.9", "numpy>=1.26"]
# ///
"""Lot RT: whole-body 2D pose (RTMW, 133 keypoints incl. 21 per hand) of a phone video.

Same video handling as ``extract_pose.py`` (rotation tag applied by hand); videos and all
outputs stay outside the (public) repository, in ``~/dev/cent-ans-mocap-src/work/rt/``::

    uv run tools/video_mocap/extract_pose_rtmw.py VIDEO.MOV OUT.npz [--mode balanced|performance]

Output ``.npz``: ``image`` (frames, 133, 2) pixel x, y of the rotated (and possibly
downscaled) frame; ``image_norm`` (frames, 133, 2) the same normalised to 0..1;
``score`` (frames, 133) RTMW keypoint score (the analogue of MediaPipe ``visibility``);
``bbox`` (frames, 4) detector box of the chosen person; ``fps``; ``size`` (width, height);
``found`` (frames,) bool. Frames without a detection repeat the previous pose.
Index layout (COCO-WholeBody): 0-16 body (COCO-17), 17-22 feet, 23-90 face, 91-111 left
hand, 112-132 right hand. "Left"/"right" are the person's, as in COCO.

Licences: rtmlib (Apache 2.0), RTMW weights and YOLOX detector from OpenMMLab mmpose
(Apache 2.0 code and weights; the weights were trained on "cocktail14", see
``docs/wip/rt-rtmw.md`` for the data-licence caveat), onnxruntime (MIT), OpenCV (Apache 2.0).
The RTMW3D / Wholebody3d weights are NOT used (trained with Human3.6M-derived data,
non-commercial). No SMPL / AMASS / HumanML3D / H36M dependency.
"""

import argparse
import sys
import time

import cv2
import numpy as np


def main() -> int:
    """Extract the whole-body keypoints of every frame of a video."""
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("video")
    ap.add_argument("out")
    ap.add_argument(
        "--mode", default="balanced", choices=["lightweight", "balanced", "performance"]
    )
    ap.add_argument("--long-side", type=int, default=1920)
    args = ap.parse_args()

    from rtmlib import Wholebody

    model = Wholebody(mode=args.mode, backend="onnxruntime", device="cpu")
    cap = cv2.VideoCapture(args.video)
    if not cap.isOpened():
        print("cannot open", args.video, file=sys.stderr)
        return 1
    fps = cap.get(cv2.CAP_PROP_FPS) or 30.0
    cap.set(cv2.CAP_PROP_ORIENTATION_AUTO, 0)
    turn = {
        90: cv2.ROTATE_90_CLOCKWISE,
        -90: cv2.ROTATE_90_COUNTERCLOCKWISE,
        270: cv2.ROTATE_90_COUNTERCLOCKWISE,
        180: cv2.ROTATE_180,
    }.get(int(round(cap.get(cv2.CAP_PROP_ORIENTATION_META))))
    pts, scs, boxes, found = [], [], [], []
    size = (0, 0)
    t0 = time.time()
    while True:
        ok, frame = cap.read()
        if not ok:
            break
        if turn is not None:
            frame = cv2.rotate(frame, turn)
        h, w = frame.shape[:2]
        scale = args.long_side / max(h, w)
        if scale < 1.0:
            frame = cv2.resize(
                frame,
                (round(w * scale), round(h * scale)),
                interpolation=cv2.INTER_AREA,
            )
        size = (frame.shape[1], frame.shape[0])
        bboxes = model.det_model(frame)
        if len(bboxes):
            # Keep the person with the largest box (the player films themselves).
            areas = (bboxes[:, 2] - bboxes[:, 0]) * (bboxes[:, 3] - bboxes[:, 1])
            box = bboxes[int(np.argmax(areas))][None, :4]
            kp, sc = model.pose_model(frame, bboxes=box)
            pts.append(kp[0])
            scs.append(sc[0])
            boxes.append(box[0])
            found.append(True)
        elif pts:
            pts.append(pts[-1])
            scs.append(np.zeros_like(scs[-1]))
            boxes.append(boxes[-1])
            found.append(False)
    if not pts:
        print("no pose found", file=sys.stderr)
        return 1
    image = np.asarray(pts, np.float32)
    np.savez_compressed(
        args.out,
        image=image,
        image_norm=image / np.asarray(size, np.float32),
        score=np.asarray(scs, np.float32),
        bbox=np.asarray(boxes, np.float32),
        found=np.asarray(found, bool),
        fps=np.float32(fps),
        size=np.asarray(size, np.int32),
    )
    print(
        f"OK {args.out} frames={len(pts)} found={sum(found)} size={size} "
        f"fps={fps:.2f} time={time.time() - t0:.1f}s"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
