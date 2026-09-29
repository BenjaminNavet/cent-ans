"""Relief cache check and ordered, resumable rebuild (lot ZG7b, ADR 0036)."""

import json
from pathlib import Path

import pytest
from typer.testing import CliRunner

from cent_ans_tools.geo import relief_cache, world_frame


def _write_manifests(map_dir: Path) -> None:
    map_dir.mkdir(parents=True, exist_ok=True)
    levels = [
        {"level": level, "tiles_rle": [{"row": 1, "runs": [[2, 2]]}]}
        for level in range(1, 8)
    ]
    (map_dir / "relief_pyramid.json").write_text(
        json.dumps(
            {"dir": "pyramid", "pattern": "E{level}/{col}_{row}.png", "levels": levels}
        )
    )
    tiles = [{"col": 3, "row": 4}]
    (map_dir / "rivers_fine.json").write_text(
        json.dumps(
            {
                "dir": "pyramid/hydro_fine",
                "pattern": "E2/{col}_{row}.bin",
                "tiles": tiles,
            }
        )
    )
    (map_dir / "fine_anchors.json").write_text(
        json.dumps(
            {
                "roads": {
                    "dir": "pyramid/roads_fine",
                    "pattern": "E2/{col}_{row}.bin",
                    "tiles": tiles,
                }
            }
        )
    )


def _bake_levels(map_dir: Path, levels: tuple[int, ...]) -> int:
    # The real bake records the frame of the cache (ADR 0119).
    world_frame.write_frame(map_dir / "pyramid", (0, 0))
    for level in levels:
        for col in (2, 3):
            path = map_dir / "pyramid" / f"E{level}" / f"{col}_1.png"
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b"x" * 10)
    return 2 * len(levels)


def _bake_fine(map_dir: Path, layer: str) -> None:
    path = map_dir / "pyramid" / layer / "E2" / "3_4.bin"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(b"y")


def _fake_runners(calls: list[tuple[str, bool]]) -> dict:
    def make(step: str, action):  # noqa: ANN001, ANN202
        def run(map_dir: Path, force: bool, workers, log) -> int:  # noqa: ANN001
            calls.append((step, force))
            return action(map_dir)

        return run

    return {
        "tier1": make("tier1", lambda d: _bake_levels(d, (1, 2))),
        "tier2": make("tier2", lambda d: _bake_levels(d, (3, 4))),
        "tier3": make("tier3", lambda d: _bake_levels(d, (5, 6, 7))),
        "hydro": make("hydro", lambda d: _bake_fine(d, "hydro_fine") or 1),
        "anchors": make("anchors", lambda d: _bake_fine(d, "roads_fine") or 1),
    }


def test_expand_rle_clips_to_the_level() -> None:
    """Runs are expanded row by row and clipped to the level's tile count."""
    rows = [{"row": 0, "runs": [[30, 5]]}, {"row": 40, "runs": [[0, 1]]}]
    assert relief_cache.expand_rle(rows, 32) == [(30, 0), (31, 0)]


def test_empty_cache_plans_every_step_in_order(tmp_path: Path) -> None:
    """No pyramid directory: every level is missing, every step is planned."""
    map_dir = tmp_path / "map"
    _write_manifests(map_dir)
    report = relief_cache.check(map_dir, tmp_path / "raw")
    assert not report.complete
    assert all(report.levels[k].missing == 2 for k in range(1, 8))
    assert report.levels[5].missing_examples == ["2_1.png", "3_1.png"]
    assert report.plan() == ["tier1", "tier2", "tier3", "hydro", "anchors"]
    assert "E1" in "\n".join(report.lines())


def test_partial_cache_plans_downstream_steps(tmp_path: Path) -> None:
    """Missing E4 reruns E3-E4, then detail-dem (it blends into E4), rivers, roads."""
    map_dir = tmp_path / "map"
    _write_manifests(map_dir)
    _bake_levels(map_dir, (1, 2, 3, 5, 6, 7))
    _bake_fine(map_dir, "hydro_fine")
    _bake_fine(map_dir, "roads_fine")
    report = relief_cache.check(map_dir, tmp_path / "raw")
    assert report.missing_steps() == ["tier2"]
    assert report.plan() == ["tier2", "tier3", "hydro", "anchors"]
    assert report.plan(force=True)[0] == "tier1"


def test_only_roads_missing_runs_anchors(tmp_path: Path) -> None:
    """A lost roads_fine directory only needs anchors-fine."""
    map_dir = tmp_path / "map"
    _write_manifests(map_dir)
    _bake_levels(map_dir, tuple(range(1, 8)))
    _bake_fine(map_dir, "hydro_fine")
    report = relief_cache.check(map_dir, tmp_path / "raw")
    assert report.plan() == ["anchors"]
    assert report.fine["roads"].missing == 1


def test_never_baked_level_is_incomplete(tmp_path: Path) -> None:
    """A level listing no tile (fresh manifest) counts as missing work."""
    map_dir = tmp_path / "map"
    _write_manifests(map_dir)
    manifest = json.loads((map_dir / "relief_pyramid.json").read_text())
    manifest["levels"][6]["tiles_rle"] = []
    (map_dir / "relief_pyramid.json").write_text(json.dumps(manifest))
    _bake_levels(map_dir, tuple(range(1, 7)))
    _bake_fine(map_dir, "hydro_fine")
    _bake_fine(map_dir, "roads_fine")
    report = relief_cache.check(map_dir, tmp_path / "raw")
    assert report.levels[7].expected == 0
    assert "jamais cuit" in "\n".join(report.lines())
    assert report.plan() == ["tier3", "hydro", "anchors"]


def test_rebuild_runs_in_order_then_resumes_with_nothing(tmp_path: Path) -> None:
    """A rebuild fills the cache; detail-dem is forced after new E3-E4; a rerun is a no-op."""
    map_dir = tmp_path / "map"
    _write_manifests(map_dir)
    calls: list[tuple[str, bool]] = []
    runners = _fake_runners(calls)
    result = relief_cache.rebuild(
        map_dir, log=lambda _: None, runners=runners, raw_dir=tmp_path / "raw"
    )
    assert [step for step, _ in calls] == [
        "tier1",
        "tier2",
        "tier3",
        "hydro",
        "anchors",
    ]
    assert dict(calls)["tier3"] is True
    assert dict(calls)["tier1"] is False
    assert result.report.complete
    calls.clear()
    again = relief_cache.rebuild(
        map_dir, log=lambda _: None, runners=runners, raw_dir=tmp_path / "raw"
    )
    assert again.ran == [] and calls == []


def test_interrupted_rebuild_resumes_at_the_failed_step(tmp_path: Path) -> None:
    """If a step dies, the next run starts from it (earlier outputs stay on disk)."""
    map_dir = tmp_path / "map"
    _write_manifests(map_dir)
    calls: list[tuple[str, bool]] = []
    runners = _fake_runners(calls)

    def crash(*_args) -> int:  # noqa: ANN002
        raise RuntimeError("coupure")

    broken = {**runners, "tier3": crash}
    with pytest.raises(RuntimeError):
        relief_cache.rebuild(
            map_dir, log=lambda _: None, runners=broken, raw_dir=tmp_path / "raw"
        )
    calls.clear()
    relief_cache.rebuild(
        map_dir, log=lambda _: None, runners=runners, raw_dir=tmp_path / "raw"
    )
    assert [step for step, _ in calls] == ["tier3", "hydro", "anchors"]
    # E3-E4 were not rewritten in this run: detail-dem simply resumes.
    assert dict(calls)["tier3"] is False


def test_cli_check_exit_code(tmp_path: Path, monkeypatch) -> None:  # noqa: ANN001
    """``geo relief-all --check`` exits 1 and names the command when incomplete."""
    from cent_ans_tools import cli

    map_dir = tmp_path / "map"
    _write_manifests(map_dir)
    real_check = relief_cache.check
    monkeypatch.setattr(
        relief_cache, "check", lambda *a, **k: real_check(map_dir, tmp_path / "raw")
    )
    result = CliRunner().invoke(cli.app, ["geo", "relief-all", "--check"])
    assert result.exit_code == 1
    assert "geo relief-all" in result.output
    _fake = _fake_runners([])
    for step in ("tier1", "tier2", "tier3", "hydro", "anchors"):
        _fake[step](map_dir, False, None, print)
    result = CliRunner().invoke(cli.app, ["geo", "relief-all", "--check"])
    assert result.exit_code == 0, result.output


def test_cache_in_the_legacy_frame_is_reframed_in_place(tmp_path: Path) -> None:
    """A cache without frame.json is in the pre-R7 frame (0, 5): renamed, not rebaked."""
    map_dir = tmp_path / "map"
    _write_manifests(map_dir)
    manifest = json.loads((map_dir / "relief_pyramid.json").read_text())
    manifest["root_origin_tiles"] = [0, 0]
    # World addresses (row 1 at E1 = legacy row 1 - 10 is out of the legacy frame:
    # use a tile that exists in both frames).
    manifest["levels"] = [
        {"level": 1, "tiles_rle": [{"row": 11, "runs": [[2, 1]]}]},
    ]
    (map_dir / "relief_pyramid.json").write_text(json.dumps(manifest))
    (map_dir / "rivers_fine.json").write_text(json.dumps({"tiles": []}))
    (map_dir / "fine_anchors.json").write_text(json.dumps({}))
    legacy = map_dir / "pyramid" / "E1" / "2_1.png"
    legacy.parent.mkdir(parents=True)
    legacy.write_bytes(b"x")
    report = relief_cache.check(map_dir, tmp_path / "raw")
    assert report.frame_shift == (0, 5)
    assert not report.complete
    calls: list[tuple[str, bool]] = []
    runners = {step: (lambda *a: 0) for step in relief_cache.RUNNERS}
    runners = {
        step: (lambda d, f, w, log, s=step: calls.append((s, f)) or 0)
        for step in relief_cache.RUNNERS
    }
    relief_cache.rebuild(map_dir, runners=runners, raw_dir=tmp_path / "raw")
    assert (map_dir / "pyramid" / "E1" / "2_11.png").exists()
    assert not legacy.exists()
    assert world_frame.cache_origin(map_dir / "pyramid") == (0, 0)
    assert relief_cache.check(map_dir, tmp_path / "raw").frame_shift is None
