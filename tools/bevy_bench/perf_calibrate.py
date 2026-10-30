#!/usr/bin/env python3
"""Writes the thresholds of one bench kind from several reports (ADR 0827).

Usage: perf_calibrate.py <thresholds.json> <map|battle> <report.json> [<report.json> ...]

The limit of a counter is the worst of the runs plus a margin (counts +5 %, main
thread CPU ms +20 %, pipelines +5 % and +2, memory +5 %); the median of the runs goes to
`reference` (the process CPU too, which is indicative only). Run it after 3+ runs of an unchanged
build, and justify any later raise in the commit.
"""

import json
import statistics
import sys
from pathlib import Path

COUNTS = [
    "draw_calls_p50",
    "draw_calls_p95",
    "triangles_p50",
    "instances_p50",
    "render_passes_p50",
]
CPU = ["main_thread_cpu_ms_p50"]
# Reported and kept as reference, never a limit: worker threads spin more when the machine is
# saturated (+30 % measured between a quiet and a saturated run), so it is not load independent.
INDICATIVE = ["process_cpu_ms_p50"]
PIPELINES = ["pipelines_created"]
PHASES = {"map": ["pan", "descent"], "battle": ["calm", "melee"]}


def main() -> int:
    """Updates the file in place."""
    conf_path, kind, *report_paths = sys.argv[1:]
    reports = [json.loads(Path(path).read_text()) for path in report_paths]
    conf = json.loads(Path(conf_path).read_text())
    limits = conf.setdefault("counters", {}).setdefault(kind, {})
    reference = conf.setdefault("reference", {}).setdefault(kind, {})
    for phase in PHASES[kind]:
        phase_limits, phase_ref = {}, {}
        rules = (
            [(s, 1.05, 0) for s in COUNTS]
            + [(s, 1.2, 0) for s in CPU]
            + [(s, 1.05, 2) for s in PIPELINES]
        )
        for stat, factor, extra in rules + [(s, None, 0) for s in INDICATIVE]:
            values = [
                r[phase]["work"][stat]
                for r in reports
                if r.get(phase, {}).get("work", {}).get(stat) is not None
            ]
            if not values:
                continue
            if factor is not None:
                phase_limits[stat] = round(max(values) * factor + extra, 2)
            phase_ref[stat] = statistics.median(values)
        limits[phase] = phase_limits
        reference[phase] = phase_ref
    memory = conf.setdefault("memory", {})
    for metric, key in (("footprint", "footprint_mib"), ("peak_rss", "peak_rss_mib")):
        values = [
            r["memory"][key] for r in reports if r.get("memory", {}).get(key, -1) > 0
        ]
        if values:
            memory.setdefault(metric, {})[kind] = round(max(values) * 1.05)
            reference.setdefault("memory", {})[key] = statistics.median(values)
    with open(conf_path, "w") as handle:
        json.dump(conf, handle, indent=2, ensure_ascii=False)
        handle.write("\n")
    print(json.dumps({"limits": limits, "reference": reference}, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
