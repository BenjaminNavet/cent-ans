"""Tests for bench.py."""

import json

import bench


def write_csv(path, rows, gpu=True):
    lines = ["frame,t_s,frame_ms,cpu_ms,gpu_ms,draw_calls"]
    for i in range(rows):
        lines.append(f"{i},{i * 0.01},{10 + i % 5},{4},{'' if not gpu else 6},{100}")
    path.write_text("\n".join(lines) + "\n")


def test_percentile():
    assert bench.percentile([], 50) is None
    assert bench.percentile([5], 95) == 5
    assert bench.percentile([1, 2, 3, 4], 50) == 2.5
    assert bench.percentile([0, 10], 95) == 9.5


def test_parse_csv_empty_fields(tmp_path):
    p = tmp_path / "a.csv"
    write_csv(p, 3, gpu=False)
    rows = bench.parse_csv(p)
    assert rows[0]["gpu_ms"] is None
    assert bench.column(rows, "gpu_ms") == []
    assert len(bench.column(rows, "frame_ms")) == 3


def test_failed_detection(tmp_path):
    assert bench.is_failed(tmp_path / "missing.csv")
    short = tmp_path / "short.csv"
    write_csv(short, 99)
    assert bench.is_failed(short)
    ok = tmp_path / "ok.csv"
    write_csv(ok, 100)
    assert not bench.is_failed(ok)


def test_report_rendering(tmp_path):
    write_csv(tmp_path / "bevy_1.csv", 200)
    write_csv(tmp_path / "godot_1.csv", 200, gpu=False)
    write_csv(tmp_path / "godot_2.csv", 10)
    (tmp_path / "passes.json").write_text(
        json.dumps(
            [
                {
                    "engine": "godot",
                    "pass": 2,
                    "csv": "godot_2.csv",
                    "failed": True,
                    "load_after": [1.5, 1, 1],
                }
            ]
        )
    )
    summary = bench.summarize(tmp_path)
    assert summary["bevy"]["frames"] == 200
    assert summary["godot"]["frames"] == 200
    assert summary["godot"]["failed_passes"] == 1
    assert summary["godot"]["gpu_ms"]["p50"] is None
    md = bench.render_markdown(summary)
    assert "n/a" in md and "| bevy | 200 |" in md


def test_log_has_fatal(tmp_path):
    ok = tmp_path / "ok.log"
    ok.write_text("PROTO godot warm-up done\n")
    assert not bench.log_has_fatal(ok)
    for marker in ("PROTO_BEVY_FATAL", "PROTO_GODOT_FATAL"):
        bad = tmp_path / f"{marker}.log"
        bad.write_text(f"x\n{marker}: scene not drawn\n")
        assert bench.log_has_fatal(bad)
    assert not bench.log_has_fatal(tmp_path / "missing.log")
