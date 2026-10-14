"""Alternating Bevy/Godot bench for the engine prototype (P0, lot A).

Commands: ``run``, ``report``, ``shots``, ``montage`` (see ``--help``).
"""

import argparse
import csv
import json
import math
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent  # tools/proto_bevy
REPO = ROOT.parent.parent
OUT_DIR = ROOT / "out"

# Launcher commands (argv prefix); the client arguments are appended.
LAUNCHERS = {
    "bevy": [str(ROOT / "bevy" / "run_bg.sh")],
    "godot": [
        str(REPO / "tools" / "godot_bg.sh"),
        "--path",
        str(ROOT / "godot"),
        "--script",
        "res://proto_bench.gd",
        "--",
    ],
}
ENGINES = ("bevy", "godot")
QUALITIES = ("comparable", "best")
SHOT_TIMES = (0, 20)
MIN_ROWS = 100
PASS_TIMEOUT_S = 600
METRICS = ("frame_ms", "cpu_ms", "gpu_ms")


def percentile(values: list[float], pct: float) -> float | None:
    """Linear-interpolated percentile (pct in 0..100); None for an empty list."""
    if not values:
        return None
    ordered = sorted(values)
    rank = (len(ordered) - 1) * pct / 100.0
    low, high = math.floor(rank), math.ceil(rank)
    if low == high:
        return ordered[low]
    return ordered[low] + (ordered[high] - ordered[low]) * (rank - low)


def parse_csv(path: Path) -> list[dict[str, float | None]]:
    """Read a metrics CSV; empty or non-numeric fields become None."""
    rows: list[dict[str, float | None]] = []
    with path.open(newline="") as handle:
        for raw in csv.DictReader(handle):
            row: dict[str, float | None] = {}
            for key, text in raw.items():
                try:
                    row[key] = float(text) if text not in (None, "") else None
                except ValueError:
                    row[key] = None
            rows.append(row)
    return rows


def column(rows: list[dict[str, float | None]], name: str) -> list[float]:
    """Non-empty values of one column."""
    return [r[name] for r in rows if r.get(name) is not None]


def is_failed(csv_path: Path) -> bool:
    """A pass fails when the CSV is missing or has fewer than MIN_ROWS rows."""
    if not csv_path.is_file():
        return True
    try:
        return len(parse_csv(csv_path)) < MIN_ROWS
    except (OSError, csv.Error):
        return True


FATAL_MARKERS = ("PROTO_BEVY_FATAL", "PROTO_GODOT_FATAL")


def log_has_fatal(log_path: Path) -> bool:
    """True when a client log contains a fatal marker (godot_bg.sh gives no exit code)."""
    try:
        text = log_path.read_text(errors="replace")
    except OSError:
        return False
    return any(marker in text for marker in FATAL_MARKERS)


def loadavg() -> list[float]:
    """Return the 1/5/15 minute load averages via sysctl (empty on failure)."""
    try:
        text = subprocess.run(
            ["sysctl", "-n", "vm.loadavg"], capture_output=True, text=True, check=True
        ).stdout
        return [
            float(t.replace(",", "."))
            for t in text.replace("{", " ").replace("}", " ").split()
        ]
    except (OSError, subprocess.CalledProcessError, ValueError):
        return []


def engine_command(engine: str, args: list[str]) -> list[str]:
    """Full argv for one client run.

    With PROTO_FOREGROUND set, both windows are on screen and frontmost (no occlusion
    workaround): Godot through godot/run_fg.sh, Bevy presenting to its window.
    """
    launcher = list(LAUNCHERS[engine])
    if os.environ.get("PROTO_FOREGROUND"):
        if engine == "godot":
            launcher[0] = str(ROOT / "godot" / "run_fg.sh")
        else:
            args = ["--set", 'bevy={"offscreen": false}', *args]
    return [*launcher, *args]


def set_args(sets: list[str]) -> list[str]:
    """Expand k=v pairs to repeated --set arguments."""
    out: list[str] = []
    for item in sets:
        out += ["--set", item]
    return out


def run_pass(
    engine: str, index: int, quality: str, sets: list[str], out_dir: Path
) -> dict:
    """Run one pass and return its record (load before/after, failed flag)."""
    csv_path = out_dir / f"{engine}_{index}.csv"
    csv_path.unlink(missing_ok=True)
    args = ["--set", f"quality={quality}", *set_args(sets), "--metrics", str(csv_path)]
    load_before = loadavg()
    started = time.time()
    try:
        proc = subprocess.run(
            engine_command(engine, args),
            capture_output=True,
            text=True,
            timeout=PASS_TIMEOUT_S,
        )
        (out_dir / f"{engine}_{index}.log").write_text(proc.stdout + proc.stderr)
    except (OSError, subprocess.TimeoutExpired) as exc:
        (out_dir / f"{engine}_{index}.log").write_text(f"launcher error: {exc!r}\n")
    return {
        "engine": engine,
        "pass": index,
        "csv": csv_path.name,
        "failed": is_failed(csv_path)
        or log_has_fatal(out_dir / f"{engine}_{index}.log"),
        "load_before": load_before,
        "load_after": loadavg(),
        "seconds": round(time.time() - started, 1),
    }


def cmd_run(args: argparse.Namespace) -> None:
    """Alternate engines for N passes each; write out/passes.json."""
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    records = []
    for index in range(1, args.passes + 1):
        for engine in ENGINES:
            print(f"[run] pass {index}/{args.passes} {engine} ...", flush=True)
            record = run_pass(engine, index, args.quality, args.set, OUT_DIR)
            status = "FAILED" if record["failed"] else "ok"
            print(
                f"[run]   {status} in {record['seconds']} s, load {record['load_after']}",
                flush=True,
            )
            records.append(record)
            (OUT_DIR / "passes.json").write_text(json.dumps(records, indent=1))


def fmt(value: float | None, digits: int = 2) -> str:
    """Format a number or 'n/a'."""
    return "n/a" if value is None else f"{value:.{digits}f}"


def summarize(out_dir: Path) -> dict:
    """Compute the per-engine summary from out/*.csv and out/passes.json."""
    passes_file = out_dir / "passes.json"
    passes = json.loads(passes_file.read_text()) if passes_file.is_file() else []
    summary: dict = {}
    for engine in ENGINES:
        rows: list[dict[str, float | None]] = []
        files = sorted(out_dir.glob(f"{engine}_*.csv"))
        failed = [p["csv"] for p in passes if p["engine"] == engine and p["failed"]]
        for path in files:
            if is_failed(path):
                if path.name not in failed:
                    failed.append(path.name)
                continue
            rows += parse_csv(path)
        entry: dict = {
            "frames": len(rows),
            "passes_ok": len(files)
            - len([f for f in failed if (out_dir / f).exists()]),
            "failed_passes": len(failed),
            "draw_calls_mean": None,
            "load_avg": [p["load_after"] for p in passes if p["engine"] == engine],
        }
        draws = column(rows, "draw_calls")
        entry["draw_calls_mean"] = sum(draws) / len(draws) if draws else None
        for metric in METRICS:
            values = column(rows, metric)
            entry[metric] = {
                "p50": percentile(values, 50),
                "p95": percentile(values, 95),
            }
        summary[engine] = entry
    return summary


def render_markdown(summary: dict) -> str:
    """Markdown table for the summary."""
    lines = [
        "| Moteur | Images | frame p50 | frame p95 | cpu p50 | cpu p95 | gpu p50 | gpu p95 | draw calls | passes échouées | charge (1 min) |",
        "|---|---|---|---|---|---|---|---|---|---|---|",
    ]
    for engine, e in summary.items():
        loads = ", ".join(fmt(load[0], 2) for load in e["load_avg"] if load) or "n/a"
        cells = [engine, str(e["frames"])]
        for metric in METRICS:
            cells += [fmt(e[metric]["p50"]), fmt(e[metric]["p95"])]
        cells += [fmt(e["draw_calls_mean"], 0), str(e["failed_passes"]), loads]
        lines.append("| " + " | ".join(cells) + " |")
    return "\n".join(lines) + "\n"


def cmd_report(args: argparse.Namespace) -> None:
    """Write out/bench.md and out/bench_summary.json."""
    summary = summarize(OUT_DIR)
    (OUT_DIR / "bench_summary.json").write_text(json.dumps(summary, indent=1))
    markdown = render_markdown(summary)
    (OUT_DIR / "bench.md").write_text(markdown)
    print(markdown)


def cmd_shots(args: argparse.Namespace) -> None:
    """Take comparable/best shots at t=0 and t=20 for each engine, sequentially."""
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    for engine in args.engine or ENGINES:
        for quality in QUALITIES:
            for at in SHOT_TIMES:
                shot = OUT_DIR / f"{engine}_{quality}_{at}.png"
                shot.unlink(missing_ok=True)
                print(f"[shots] {engine} {quality} t={at}", flush=True)
                cmd = engine_command(
                    engine,
                    [
                        "--set",
                        f"quality={quality}",
                        "--shot",
                        str(shot),
                        "--at",
                        str(at),
                    ],
                )
                try:
                    subprocess.run(
                        cmd, capture_output=True, text=True, timeout=PASS_TIMEOUT_S
                    )
                except (OSError, subprocess.TimeoutExpired) as exc:
                    print(f"[shots]   error: {exc!r}", file=sys.stderr)
                print(f"[shots]   {'ok' if shot.is_file() else 'MISSING'}", flush=True)


def cmd_montage(args: argparse.Namespace) -> None:
    """Assemble Bevy|Godot side-by-side images (Pillow)."""
    from PIL import Image

    for quality in QUALITIES:
        for at in SHOT_TIMES:
            left = OUT_DIR / f"bevy_{quality}_{at}.png"
            right = OUT_DIR / f"godot_{quality}_{at}.png"
            if not (left.is_file() and right.is_file()):
                print(f"[montage] skip {quality} t={at} (missing shot)")
                continue
            with Image.open(left) as a, Image.open(right) as b:
                canvas = Image.new("RGB", (a.width + b.width, max(a.height, b.height)))
                canvas.paste(a.convert("RGB"), (0, 0))
                canvas.paste(b.convert("RGB"), (a.width, 0))
                target = OUT_DIR / f"compare_{quality}_{at}.png"
                canvas.save(target)
            print(f"[montage] {target.name}")


def build_parser() -> argparse.ArgumentParser:
    """Build the CLI parser."""
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    run = sub.add_parser("run", help="Alternate bevy/godot passes.")
    run.add_argument("--passes", type=int, default=3, help="Passes per engine.")
    run.add_argument("--quality", default="comparable", choices=QUALITIES)
    run.add_argument("--set", action="append", default=[], metavar="K=V")
    run.set_defaults(func=cmd_run)
    sub.add_parser(
        "report", help="Write out/bench.md and bench_summary.json."
    ).set_defaults(func=cmd_report)
    shots = sub.add_parser("shots", help="Take the 8 comparison shots.")
    shots.add_argument(
        "--engine", action="append", choices=ENGINES, help="Only these engines."
    )
    shots.set_defaults(func=cmd_shots)
    sub.add_parser("montage", help="Build side-by-side images.").set_defaults(
        func=cmd_montage
    )
    return parser


def main() -> None:
    """Entry point."""
    args = build_parser().parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
