"""Automatic local fallback of ``tools/experiments/dn_batch.py`` with fal simulated as failing."""

import importlib.util
import json
import sys
from pathlib import Path

import pytest

SCRIPT = Path(__file__).resolve().parents[1] / "experiments" / "dn_batch.py"


@pytest.fixture
def batch(tmp_path, monkeypatch):
    """The module with its output tree redirected to a temporary folder."""
    spec = importlib.util.spec_from_file_location("dn_batch_under_test", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    monkeypatch.setattr(module, "DN", tmp_path)
    monkeypatch.setattr(module, "LOCK_PATH", tmp_path / "gpu.lock")
    return module


def test_fal_failure_falls_back_to_sf3d(batch, tmp_path, monkeypatch):
    """Fal raises, HF has no quota: SF3D local runs and the used backend is recorded."""
    entry = {"id": "obj", "kind": "decor", "ingest": {"class": "prop"}}
    out_dir = tmp_path / "obj"
    (out_dir / "cut").mkdir(parents=True)
    (out_dir / "cut" / "s1337.png").write_bytes(b"png")

    def failing_fal(*_args, **_kwargs):
        raise RuntimeError("content filter")

    def no_quota(*_args, **_kwargs):
        return None

    def fake_sf3d(_entry_id, _cut, target, _seed):
        target.write_bytes(b"glb")

    monkeypatch.setattr(batch, "fal_trellis", failing_fal)
    monkeypatch.setattr(batch, "hf_trellis", no_quota)
    monkeypatch.setattr(batch, "sf3d_run", fake_sf3d)
    glb = out_dir / "3d" / "fal__s1337.glb"
    (out_dir / "3d").mkdir()
    batch.fal_job("obj", out_dir / "cut" / "s1337.png", glb, 1337, entry)
    assert not glb.exists()
    assert batch.local_3d_fallback(entry, out_dir, 1337) == "sf3d"
    assert (out_dir / "3d" / "sf3d__s1337.glb").exists()
    failures = (tmp_path / "failures.jsonl").read_text()
    assert "fal" in failures
    calls = json.loads((out_dir / "generation.json").read_text())["calls_3d"]
    assert calls[-1]["endpoint"] == "sf3d-local"
