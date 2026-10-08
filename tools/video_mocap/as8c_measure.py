# /// script
# requires-python = ">=3.10,<3.13"
# dependencies = ["opencv-python-headless>=4.9", "numpy>=1.26"]
# ///
"""Lot AS8c: measured motion of beasts and carts from free videos (ADR 0189).

Runs ``measure_motion.py`` on a fixed list of clips (point positions picked by eye on contact
sheets, see ``CLIPS``) and writes the numbers that feed ``data/fx/animal_motion.json`` to
``data/fx/animal_motion_measured.json`` (CC0 videos) and ``data/fx/camp_horse_motion.json``
(``horse_rest`` and the Rama clips: CC BY-SA 2.0 fr, kept in their own file). Videos stay outside
the repository::

    uv run tools/video_mocap/as8c_measure.py [--src ~/dev/cent-ans-mocap-src/video/free/animals] [--out data/fx/animal_motion_measured.json]

Intermediate tracks go to ``<src>/_work``. Seeking in the long WebM files is slow (a few
minutes for the whole run). Units: image px are converted with the scale of each clip, given
as ``px_per_m`` (withers height of the breed for cattle and horses, rear wheel radius for the
wagon) -- those scales are assumptions, listed in the output under ``assumptions``.
"""

import argparse
import json
import subprocess
import sys
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
TOOL = HERE / "measure_motion.py"

# name -> (video, kind, spec). Points are source pixels at ``start_s``.
CLIPS = {
    "cow_walk": (
        "boissia/boissia.webm",
        "track",
        {
            "start_s": 1.3,
            "end_s": 4.0,
            "body_px": 200,
            "reference": "hip",
            "background": "post",
            "legs": ["shank_r"],
            "f0": 0.71,
            "angles": {"head": ["withers", "ear"]},
            "mil": {"shank_f": 34, "shank_r": 34, "hip": 50, "withers": 50},
            "points": {
                "hip": [550, 449],
                "withers": [444, 423],
                "ear": [406, 432],
                "shank_f": [425, 539],
                "shank_r": [556, 539],
                "post": [270, 406],
            },
        },
    ),
    "cow_speed": (
        "boissia/boissia.webm",
        "track",
        {
            "start_s": 1.3,
            "end_s": 2.4,
            "body_px": 150,
            "reference": "hip",
            "f0": 0.85,
            "legs": [],
            "ground": ["g1", "g2", "g3"],
            "mil": {"hip": 50},
            "points": {
                "hip": [550, 449],
                "g1": [330, 612],
                "g2": [470, 618],
                "g3": [610, 600],
            },
        },
    ),
    "cow_period_a": (
        "boissia/boissia.webm",
        "period",
        {
            "start_s": 1.3,
            "end_s": 5.0,
            "box": [395, 395, 190, 190],
            "band": [0.6, 1.0],
            "max_lag_s": 2.2,
        },
    ),
    "cow_period_b": (
        "boissia/boissia.webm",
        "period",
        {
            "start_s": 1.3,
            "end_s": 4.5,
            "box": [225, 400, 185, 170],
            "band": [0.6, 1.0],
            "max_lag_s": 2.2,
        },
    ),
    "cow_graze": (
        "boissia/boissia.webm",
        "track",
        {
            "start_s": 57.5,
            "end_s": 62.0,
            "body_px": 170,
            "reference": "withers",
            "f0": 1.0,
            "legs": [],
            "fmin": 0.3,
            "fmax": 3.0,
            "angles": {"head": ["withers", "poll"]},
            "mil": {"poll": 40, "muzzle": 30, "rump": 50, "withers": 50},
            "points": {
                "withers": [200, 346],
                "rump": [300, 359],
                "poll": [137, 427],
                "muzzle": [119, 484],
            },
        },
    ),
    "sheep_graze": (
        "sheep_elbe/sheep_elbe.webm",
        "track",
        {
            "start_s": 41.0,
            "end_s": 58.0,
            "body_px": 230,
            "reference": "withers",
            "f0": 0.5,
            "legs": [],
            "fmin": 0.2,
            "fmax": 3.0,
            "angles": {"head": ["withers", "poll"]},
            "mil": {"poll": 44, "nose": 32, "rump": 50, "withers": 50},
            "points": {
                "withers": [255, 85],
                "rump": [128, 108],
                "poll": [320, 140],
                "nose": [350, 185],
            },
        },
    ),
    "horse_stand": (
        "rama7491/rama7491.ogv",
        "track",
        {
            "start_s": 0.1,
            "end_s": 7.0,
            "body_px": 480,
            "reference": "withers",
            "f0": 0.35,
            "legs": [],
            "fmin": 0.1,
            "fmax": 1.0,
            "angles": {"head": ["withers", "poll"], "tail": ["rump", "tail"]},
            "mil": {"tail": 36, "hoof_f": 30, "hoof_h": 30, "poll": 40},
            "points": {
                "withers": [300, 130],
                "rump": [540, 140],
                "poll": [170, 124],
                "tail": [588, 350],
                "hoof_f": [356, 368],
                "hoof_h": [540, 368],
                "barrel": [400, 230],
                "flank": [470, 200],
            },
        },
    ),
    "horse_chew": (
        "rama7493/rama7493.ogv",
        "track",
        {
            "start_s": 0.3,
            "end_s": 6.4,
            "body_px": 480,
            "reference": "eye",
            "f0": 1.5,
            "legs": [],
            "fmin": 0.6,
            "fmax": 3.5,
            "angles": {"jaw": ["eye", "chin"]},
            "mil": {"chin": 40, "eye": 30},
            "points": {"eye": [380, 156], "chin": [395, 345], "nose": [420, 322]},
        },
    ),
    "horse_eat": (
        "rama7496/rama7496.ogv",
        "track",
        {
            "start_s": 10.0,
            "end_s": 18.0,
            "body_px": 480,
            "reference": "withers",
            "f0": 1.0,
            "legs": [],
            "angles": {"head": ["withers", "poll"]},
            "mil": {"poll": 40, "nose": 36, "tail": 36},
            "points": {
                "withers": [200, 92],
                "poll": [512, 98],
                "nose": [540, 205],
                "tail": [70, 380],
                "rump": [60, 140],
            },
        },
    ),
    "wagon_box": (
        "wagons/wagons.webm",
        "track",
        {
            "start_s": 6.5,
            "end_s": 8.4,
            "body_px": 1000,
            "reference": "hub_f",
            "f0": 1.0,
            "legs": ["box_b"],
            "angles": {
                "box_roll": ["box_b", "rail_c"],
                "axle_pitch": ["hub_f", "hub_r"],
            },
            "points": {
                "hub_f": [1130, 792],
                "hub_r": [1412, 725],
                "box_b": [950, 612],
                "rail_c": [1300, 520],
            },
        },
    ),
    "wagon_speed": (
        "wagons/wagons.webm",
        "track",
        {
            "start_s": 6.5,
            "end_s": 7.3,
            "body_px": 208,
            "reference": "hub_f",
            "f0": 1.0,
            "legs": [],
            "ground": ["g2", "g3"],
            "points": {
                "hub_f": [1130, 792],
                "g1": [1050, 905],
                "g2": [1300, 910],
                "g3": [1180, 930],
            },
        },
    ),
    "wagon_horses": (
        "wagons/wagons.webm",
        "period",
        {
            "start_s": 6.5,
            "end_s": 9.5,
            "box": [230, 450, 560, 420],
            "band": [0.62, 1.0],
            "max_lag_s": 2.0,
            "min_period_s": 0.3,
        },
    ),
}

ASSUMPTIONS = {
    "cattle_withers_m": 1.4,
    "horse_withers_m": 1.7,
    "wagon_px_per_m": 208,
    "note": "Les echelles viennent de la taille de la race (garrot) ou du rayon de roue (0,6 m) : hypotheses, pas des mesures.",
}


def run(*args):
    """Run."""
    return subprocess.run(
        ["uv", "run", str(TOOL), *map(str, args)],
        check=True,
        capture_output=True,
        text=True,
    ).stdout


def work_paths(src, name):
    """Work paths."""
    work = src / "_work"
    work.mkdir(exist_ok=True)
    return work / f"{name}.json", work / f"{name}.npz", work / f"{name}_res.json"


def measure_clip(src, name):
    """Measure clip."""
    video, kind, spec = CLIPS[name]
    spec_path, npz_path, res_path = work_paths(src, name)
    spec_path.write_text(json.dumps(spec))
    if kind == "period":
        run("period", src / video, spec_path, "--json", res_path)
        return json.loads(res_path.read_text())
    if not npz_path.exists():
        run("track", src / video, spec_path, npz_path)
    run("analyse", npz_path, spec_path, "--json", res_path)
    return json.loads(res_path.read_text())


def load_track(src, name):
    """Load track."""
    npz = np.load(work_paths(src, name)[1])
    return {
        "names": [str(n) for n in npz["names"]],
        "traj": npz["traj"].astype(float),
        "fps": float(npz["fps"]),
    }


def rel(track, a, b, axis):
    """Rel."""
    names = track["names"]
    return (
        track["traj"][:, names.index(b), axis] - track["traj"][:, names.index(a), axis]
    )


def high_pass(signal, fps, seconds=1.0):
    """High pass."""
    win = max(3, int(seconds * fps))
    slow = np.convolve(
        np.pad(signal, win // 2, mode="edge"), np.ones(win) / win, mode="same"
    )[win // 2 : win // 2 + len(signal)]
    return signal - slow


def spectrum_peaks(signal, fps, lo, hi, count=3):
    """Strongest spectral peaks in [lo, hi] Hz: [(Hz, ratio to the band mean)]."""
    s = high_pass(signal, fps)
    n = len(s)
    pad = 1 << (n * 4 - 1).bit_length()
    spec = np.abs(np.fft.rfft(s * np.hanning(n), pad))
    freqs = np.fft.rfftfreq(pad, 1 / fps)
    band = (freqs >= lo) & (freqs <= hi)
    peaks = []
    for k in np.argsort(spec * band)[::-1]:
        if band[k] and all(abs(freqs[k] - p) > 0.15 for p, _ in peaks):
            peaks.append(
                (
                    round(float(freqs[k]), 3),
                    round(float(spec[k] / spec[band].mean()), 1),
                )
            )
        if len(peaks) == count:
            break
    return peaks, float(s.std())


def pitch_deg(track, a, b, sign):
    """Angle of the segment a->b below the horizontal (deg); ``sign`` -1 when the head points to image-left."""
    return np.degrees(np.arctan2(rel(track, a, b, 1), sign * rel(track, a, b, 0)))


def circular_smooth(values):
    """Circular smooth."""
    v = np.asarray(values, float)
    return (np.roll(v, 1) + 2 * v + np.roll(v, -1)) / 4


def gait_luts(res, leg):
    """Forward-displacement and lift look-up tables (16 samples, unit amplitude) from a measured leg.

    ``x_rel_ref`` of the leg is positive backwards (the animal walks to image-left in the clip),
    ``y_rel_ref`` positive up. Phase 0 of the tables is the mid-swing instant (forward
    displacement crossing zero while rising), where the shader starts its leg cycle.
    """
    back = circular_smooth(res["points"][leg]["x_rel_ref"]["profile16"])
    up = circular_smooth(res["points"][leg]["y_rel_ref"]["profile16"])
    forward = -back
    amp = (forward.max() - forward.min()) / 2
    crossings = [i for i in range(16) if forward[i] < 0 <= forward[(i + 1) % 16]]
    start = (
        crossings[0]
        + (
            -forward[crossings[0]]
            / (forward[(crossings[0] + 1) % 16] - forward[crossings[0]])
        )
        if crossings
        else 0.0
    )
    grid = (start + np.arange(16)) % 16
    xs = np.arange(17)
    fwd = np.interp(grid, xs, np.append(forward, forward[0])) / amp
    lift = np.interp(grid, xs, np.append(up, up[0]))
    lift = (lift - lift.min()) / (lift.max() - lift.min())
    return [round(float(v), 3) for v in fwd], [round(float(v), 3) for v in lift]


def derive(src):
    """Derive."""
    out = {"assumptions": ASSUMPTIONS, "clips": {}}
    res = {name: measure_clip(src, name) for name in CLIPS}
    tracks = {
        name: load_track(src, name)
        for name, (_, kind, _) in CLIPS.items()
        if kind == "track"
    }
    for name in res:
        out["clips"][name] = {
            "video": CLIPS[name][0],
            "start_s": CLIPS[name][2]["start_s"],
            "end_s": CLIPS[name][2]["end_s"],
        }

    # Cattle (Boissia herd, CC0): cadence, speed, stride, head, bites.
    periods = [res["cow_period_a"]["period_s"], res["cow_period_b"]["period_s"]]
    cadence = float(np.mean([1 / p for p in periods]))
    speed_w = abs(res["cow_speed"]["speed_body_per_s"])
    stride_w = speed_w / cadence
    walk_head = pitch_deg(tracks["cow_walk"], "withers", "ear", -1)
    graze_head = pitch_deg(tracks["cow_graze"], "withers", "poll", -1)
    bite_peaks, _ = spectrum_peaks(
        rel(tracks["cow_graze"], "poll", "muzzle", 1),
        tracks["cow_graze"]["fps"],
        0.5,
        3.0,
    )
    _, bite_rms = spectrum_peaks(
        rel(tracks["cow_graze"], "withers", "poll", 1),
        tracks["cow_graze"]["fps"],
        0.5,
        3.0,
    )
    head_len = float(
        np.hypot(
            rel(tracks["cow_graze"], "withers", "poll", 0),
            rel(tracks["cow_graze"], "withers", "poll", 1),
        ).mean()
    )
    fwd, lift = gait_luts(res["cow_walk"], "shank_r")
    out["cattle"] = {
        "source": "boissia (CC0)",
        "stride_period_s": periods,
        "cadence_hz": round(cadence, 3),
        "speed_withers_per_s": round(speed_w, 3),
        "stride_withers": round(stride_w, 3),
        "walk_head_below_horizontal_deg": round(float(walk_head.mean()), 1),
        "head_nod_half_rad": res["cow_walk"]["angles"]["head"]["p5_p95_half"],
        "graze_head_below_horizontal_deg": round(float(graze_head.mean()), 1),
        "graze_delta_rad": round(
            float(np.radians(graze_head.mean() - walk_head.mean())), 3
        ),
        "bite_peaks_hz": bite_peaks,
        "bite_amp_rad": round(float(np.sqrt(2) * bite_rms / head_len), 3),
        "shank_rear_swing_half_withers": round(
            res["cow_walk"]["points"]["shank_r"]["x_rel_ref"]["p5_p95_half"]
            * 200
            / 150,
            3,
        ),
        "swing_lut": fwd,
        "lift_lut": lift,
    }
    # Sheep (Elbe, CC0).
    sheep_peaks, sheep_rms = spectrum_peaks(
        rel(tracks["sheep_graze"], "poll", "nose", 1),
        tracks["sheep_graze"]["fps"],
        0.6,
        3.0,
    )
    step_peaks, _ = spectrum_peaks(
        rel(tracks["sheep_graze"], "withers", "rump", 1),
        tracks["sheep_graze"]["fps"],
        0.6,
        3.0,
    )
    sheep_head = pitch_deg(tracks["sheep_graze"], "withers", "poll", 1)
    sheep_len = float(
        np.hypot(
            rel(tracks["sheep_graze"], "withers", "poll", 0),
            rel(tracks["sheep_graze"], "withers", "poll", 1),
        ).mean()
    )
    wp_peaks, wp_rms = spectrum_peaks(
        rel(tracks["sheep_graze"], "withers", "poll", 1),
        tracks["sheep_graze"]["fps"],
        0.6,
        3.0,
    )
    out["sheep"] = {
        "source": "sheep_elbe (CC0)",
        "graze_head_below_horizontal_deg": round(float(sheep_head.mean()), 1),
        "graze_delta_rad": round(
            float(np.radians(sheep_head.mean() - walk_head.mean())), 3
        ),
        "head_down_share": round(float((sheep_head > 30).mean()), 2),
        "bite_peaks_hz": sheep_peaks,
        "bite_amp_rad": round(float(np.sqrt(2) * wp_rms / sheep_len), 3),
        "step_peaks_hz": step_peaks,
    }
    # Horse at rest (Rama, CC BY-SA 2.0 fr).
    chew_peaks, chew_rms = spectrum_peaks(
        rel(tracks["horse_chew"], "eye", "chin", 1),
        tracks["horse_chew"]["fps"],
        0.5,
        3.0,
    )
    jaw_len = float(
        np.hypot(
            rel(tracks["horse_chew"], "eye", "chin", 0),
            rel(tracks["horse_chew"], "eye", "chin", 1),
        ).mean()
    )
    eat_head = pitch_deg(tracks["horse_eat"], "withers", "poll", 1)
    times = np.arange(len(eat_head)) / tracks["horse_eat"]["fps"]
    lo, hi = np.percentile(eat_head, [5, 95])
    t10 = times[np.argmax(eat_head > lo + 0.1 * (hi - lo))]
    t90 = times[np.argmax(eat_head > lo + 0.9 * (hi - lo))]
    tail = res["horse_stand"]["angles"]["tail"]
    out["horse_rest"] = {
        "source": "rama7491/7493/7496 (CC BY-SA 2.0 fr)",
        "chew_peaks_hz": chew_peaks,
        "chew_amp_rad": round(float(np.sqrt(2) * chew_rms / jaw_len), 3),
        "head_up_deg": round(float(lo), 1),
        "head_down_deg": round(float(hi), 1),
        "head_lift_rad": round(float(np.radians(hi - lo)), 3),
        "head_lowering_s": round(float(t90 - t10), 2),
        "tail_swing_half_rad": tail["p5_p95_half"],
        "tail_hz": tail["own_hz"],
        "stand_hoof_motion_half_body": max(
            res["horse_stand"]["points"][p]["x_rel_ref"]["p5_p95_half"]
            for p in ("hoof_f", "hoof_h")
        ),
        "stand_barrel_half_body": res["horse_stand"]["points"]["barrel"]["y_rel_ref"][
            "p5_p95_half"
        ],
        "note": "Aucun report de poids ni souffle resolu en 7 s (< 0,5 px) : bornes superieures seulement.",
    }
    # Wagon (CC0).
    px_per_m = ASSUMPTIONS["wagon_px_per_m"]
    wagon_speed = (
        abs(res["wagon_speed"]["speed_body_per_s"])
        * res["wagon_speed"]["body_px"]
        / px_per_m
    )
    jolt_peaks, jolt_rms = spectrum_peaks(
        rel(tracks["wagon_box"], "hub_f", "box_b", 1),
        tracks["wagon_box"]["fps"],
        0.5,
        8.0,
    )
    rail_peaks, rail_rms = spectrum_peaks(
        rel(tracks["wagon_box"], "hub_f", "rail_c", 1),
        tracks["wagon_box"]["fps"],
        0.5,
        8.0,
    )
    rms_m = float(np.mean([jolt_rms, rail_rms])) / px_per_m
    weights = np.array([w for _, w in jolt_peaks])
    lines = [
        {
            "hz": hz,
            "spatial_period_m": round(wagon_speed / hz, 2),
            "amp_m": round(
                float(np.sqrt(2) * rms_m * w / np.sqrt((weights**2).sum())), 4
            ),
        }
        for hz, w in jolt_peaks
    ]
    out["wagon"] = {
        "source": "wagons (CC0)",
        "speed_m_per_s": round(wagon_speed, 2),
        "box_rms_m": round(rms_m, 4),
        "jolt_lines": lines,
        "roll_half_rad": res["wagon_box"]["angles"]["box_roll"]["p5_p95_half"],
        "roll_hz": res["wagon_box"]["angles"]["box_roll"]["own_hz"],
        "horse_half_cycle_s": res["wagon_horses"].get("period_s"),
        "horse_stride_period_s": round(2 * res["wagon_horses"]["period_s"], 2),
        "horse_stride_m": round(wagon_speed * 2 * res["wagon_horses"]["period_s"], 2),
    }
    return out


def main():
    """Main."""
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "--src", default=str(Path.home() / "dev/cent-ans-mocap-src/video/free/animals")
    )
    parser.add_argument(
        "--out", default=str(HERE.parent.parent / "data/fx/animal_motion_measured.json")
    )
    parser.add_argument(
        "--camp-out", default=str(HERE.parent.parent / "data/fx/camp_horse_motion.json")
    )
    args = parser.parse_args()
    result = derive(Path(args.src))
    # The Rama videos are CC BY-SA 2.0 fr: their numbers go to their own data file, so that
    # animal_motion*.json stay CC0-derived (ADR 0189).
    rest = result.pop("horse_rest")
    rest.pop("source", None)
    rama_clips = {
        name: result["clips"].pop(name)
        for name in ("horse_stand", "horse_chew", "horse_eat")
    }
    Path(args.out).write_text(
        json.dumps(result, indent=1, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    camp_path = Path(args.camp_out)
    camp = json.loads(camp_path.read_text(encoding="utf-8"))
    camp["horse_rest"] = rest
    camp["clips"] = rama_clips
    camp_path.write_text(
        json.dumps(camp, indent=1, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    result["horse_rest"] = rest
    json.dump(result, sys.stdout, indent=1, ensure_ascii=False)


if __name__ == "__main__":
    main()
