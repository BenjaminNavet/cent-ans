"""Lot AS8b: footfall measures of the baked horse clips (A/B between the gaits).

    [AS8B_LEGACY=1] blender -b --factory-startup --python tools/blender_scripts/as8b_measure.py \
        -- c_trot c_gallop c_walk [cadence overrides name=speed]

For each clip: the horse is posed frame by frame exactly as `bake_cavalry_rig` does, then per
leg the hoof (IK controller) is followed. A hoof is planted when it is within 4 cm of its
standing height. While planted it must travel backwards in the body frame at the clip's
cadence (m/s, `data/fx/battle_gore.json`): the slip is the mean gap, in m/s and in % of the
cadence. Also: planted share per leg, frames with no hoof planted (suspension), cadence that
would remove the slip (median planted speed), and the deepest hoof below the standing height.
"""

import json
import os
import statistics
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import battle_skinned as bs  # noqa: E402
import battle_skinned_cavalry as cav  # noqa: E402
import battle_skinned_poses as poses  # noqa: E402
import bpy  # noqa: E402

FPS = 24.0
PLANTED_TOL = 0.015


def cadences():
    path = os.path.join(bs.ROOT, "data", "fx", "battle_gore.json")
    with open(path) as f:
        return json.load(f)["cadence"]


def measure(mount, row, cadence, rest):
    clip, horse_act, rider_act, pose, _mirror, frames = row
    harm = mount.harm
    h_act = bs.find_action(horse_act, "AnimalArmature")
    bs.set_action(harm, h_act)
    first, last = (int(round(v)) for v in h_act.frame_range)
    length = last - first + 1
    count = frames or length
    track = {k: [] for k in poses.HORSE_LEGS}
    wrong = 0
    for i in range(count):
        t = i / max(count - 1, 1)
        for pb in harm.pose.bones:
            pb.matrix_basis.identity()
        bpy.context.scene.frame_set(first + i % length)
        poses.reset_state()
        horse = getattr(pose, "horse", None)
        if horse is not None:
            horse(harm, t)
        bpy.context.view_layer.update()
        for key, (chain, hoof) in poses.HORSE_LEGS.items():
            track[key].append(poses.pos(harm, hoof))
            # Joint on the wrong side of the root-hoof line (fore knee behind, hind hock ahead).
            root, knee = poses.pos(harm, chain[0]), poses.pos(harm, chain[-1])
            tip = poses.pos(harm, hoof)
            f = (knee.z - tip.z) / max(root.z - tip.z, 1e-6)
            line_y = tip.y + (root.y - tip.y) * f
            side = (line_y - knee.y) if key[0] == "F" else (knee.y - line_y)  # + = correct
            if side < -0.02:
                wrong += 1
                if os.environ.get("AS8B_DUMP"):
                    print("WRONG", clip, key, i, round(side, 3))
    if os.environ.get("AS8B_DUMP"):
        for key, pts in track.items():
            print("DUMP", clip, key, [(round(p.y, 2), round(p.z - rest[key].z, 2)) for p in pts])
    planted = {}
    speeds = []
    below = 0.0
    for key, pts in track.items():
        flags = []
        for p in pts:
            flags.append(p.z - rest[key].z < PLANTED_TOL)
            below = max(below, rest[key].z - p.z)
        planted[key] = flags
        n = len(pts)
        for i in range(n):
            j = (i + 1) % n
            if flags[i] and flags[j]:
                # Body frame: the ground slides backwards (+Y, the horse faces -Y).
                speeds.append((pts[j].y - pts[i].y) * FPS)
    duty = {k: round(sum(v) / len(v), 2) for k, v in planted.items()}
    air = sum(1 for i in range(count) if not any(planted[k][i] for k in planted))
    out = {
        "clip": clip,
        "frames": count,
        "duty": duty,
        "air_frames": air,
        "below_m": round(below, 3),
        "wrong_joint_frames": wrong,
    }
    if speeds:
        out["planted_speed_median"] = round(statistics.median(speeds), 2)
        if cadence:
            slip = statistics.mean(abs(s - cadence) for s in speeds)
            out["cadence"] = cadence
            out["slip_mps"] = round(slip, 2)
            med = statistics.median(speeds)
            out["self_slip_pct"] = round(100 * statistics.mean(abs(s - med) for s in speeds) / max(abs(med), 1e-6))
            out["slip_pct"] = round(100 * slip / cadence)
    return out


def main():
    args = sys.argv[sys.argv.index("--") + 1 :]
    names = [a for a in args if "=" not in a]
    forced = dict(a.split("=") for a in args if "=" in a)
    table = cadences()
    mount = cav.Mount()
    rest = {k: poses.pos(mount.harm, hoof) for k, (_c, hoof) in poses.HORSE_LEGS.items()}
    rows = {r[0]: r for r in cav.clip_specs()}
    for name in names:
        cad = float(forced.get(name, table.get(name, 0)))
        print("MEASURE", json.dumps(measure(mount, rows[name], cad, rest)))


main()
