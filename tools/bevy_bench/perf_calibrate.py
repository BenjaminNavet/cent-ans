#!/usr/bin/env python3
"""Writes the thresholds of one bench kind from several reports (ADR 0827).

Usage: perf_calibrate.py <thresholds.json> <map|battle> <report.json> [<report.json> ...]

The limit of a counter is the worst of the runs plus a margin (counts +5 %, CPU ms +15 %,
pipelines +5 % and +2, memory +5 %); the median of the runs goes to `reference`. Run it after 3
guard runs of an unchanged build, and justify any later raise in the commit.
"""

import json
import statistics
import sys

COUNTS = ["draw_calls_p50", "draw_calls_p95", "triangles_p50", "instances_p50", "render_passes_p50"]
CPU = ["process_cpu_ms_p50", "main_thread_cpu_ms_p50"]
PIPELINES = ["pipelines_created"]
PHASES = {"map": ["pan", "descent"], "battle": ["calm", "melee"]}


def main() -> int:
    """Updates the file in place."""
    conf_path, kind, *report_paths = sys.argv[1:]
    reports = [json.load(open(path)) for path in report_paths]
    conf = json.load(open(conf_path))
    limits = conf.setdefault("counters", {}).setdefault(kind, {})
    reference = conf.setdefault("reference", {}).setdefault(kind, {})
    for phase in PHASES[kind]:
        phase_limits, phase_ref = {}, {}
        rules = [(s, 1.05, 0) for s in COUNTS] + [(s, 1.15, 0) for s in CPU] + [(s, 1.05, 2) for s in PIPELINES]
        for stat, factor, extra in rules:
            values = [r[phase]["work"][stat] for r in reports if r.get(phase, {}).get("work", {}).get(stat) is not None]
            if not values:
                continue
            phase_limits[stat] = round(max(values) * factor + extra, 2)
            phase_ref[stat] = statistics.median(values)
        limits[phase] = phase_limits
        reference[phase] = phase_ref
    memory = conf.setdefault("memory", {})
    for metric, key in (("footprint", "footprint_mib"), ("peak_rss", "peak_rss_mib")):
        values = [r["memory"][key] for r in reports if r.get("memory", {}).get(key, -1) > 0]
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
