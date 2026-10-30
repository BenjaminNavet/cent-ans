#!/usr/bin/env python3
"""Compares one bench report with the work-counter thresholds (perf_thresholds.json v2, ADR 0827).

Usage: perf_check.py <thresholds.json> <map|battle> <report.json>
Exit 0 when every counter is within its limit, 1 otherwise (a counter that is absent or null
counts as a failure: the build lacks the `perf-counters` feature or the report is stale).
"""

import json
import sys
from pathlib import Path


def main() -> int:
    """Prints one line per checked counter, returns the process exit code."""
    conf_path, kind, report_path = sys.argv[1:4]
    conf = json.loads(Path(conf_path).read_text())
    report = json.loads(Path(report_path).read_text())
    bad = 0
    for phase, limits in conf["counters"][kind].items():
        if phase.startswith("_"):
            continue
        work = (report.get(phase) or {}).get("work")
        if not work:
            print(f"FAIL {kind}.{phase}: no work counters in the report")
            bad += 1
            continue
        frame = report[phase]
        print(
            f"info {kind}.{phase}: frame p50 {frame['frame_ms_p50']} ms, p95 {frame['frame_ms_p95']} ms (not compared)"
        )
        for stat, limit in limits.items():
            value = work.get(stat)
            if value is None:
                print(
                    f"FAIL {kind}.{phase}.{stat}: absent or null (build without perf-counters?)"
                )
                bad += 1
                continue
            ok = value <= limit
            bad += not ok
            print(
                f"{'ok  ' if ok else 'FAIL'} {kind}.{phase}.{stat} = {value} (max {limit})"
            )
    memory_conf = conf.get("memory", {})
    for metric, key in (("footprint", "footprint_mib"), ("peak_rss", "peak_rss_mib")):
        limit = memory_conf.get(metric, {}).get(kind)
        if limit is None:
            continue
        value = report.get("memory", {}).get(key)
        if value is None or value < 0:
            print(f"FAIL {kind}.{metric}: absent from the report")
            bad += 1
        else:
            ok = value <= limit
            bad += not ok
            print(
                f"{'ok  ' if ok else 'FAIL'} {kind}.{metric} = {value:.0f} MiB (max {limit})"
            )
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
