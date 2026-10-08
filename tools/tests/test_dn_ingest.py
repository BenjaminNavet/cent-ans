"""Unit tests for the DN ingest tool (Python side only; Blender is not run)."""

import json
import sys
from pathlib import Path

import numpy as np
import pytest

from cent_ans_tools import dn_ingest
from tests.conftest import assert_matches_schema

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "blender_scripts"))
import dn_grade  # noqa: E402

DATA = Path(__file__).resolve().parents[2] / "data"


def _job(**overrides):
    args = {
        "asset_id": "house_a",
        "asset_class": "house",
        "raw": Path("/raw/x.glb"),
        "classes": dn_ingest.load_classes(),
        "length": 8.0,
    }
    args.update(overrides)
    return dn_ingest.build_job(**args)


def test_data_files_match_schemas() -> None:
    """Class table and manifest validate against their schemas."""
    assert_matches_schema(
        DATA / "art" / "dn_ingest_classes.json", "art_dn_ingest_classes.schema.json"
    )
    assert_matches_schema(
        DATA / "art" / "dn_manifest.json", "art_dn_manifest.schema.json"
    )


def test_class_budgets_follow_bible() -> None:
    """Budgets decrease per LOD and match the bible 14.4 table for the main classes."""
    classes = dn_ingest.load_classes()["classes"]
    for spec in classes.values():
        assert spec["lod_tris"] == sorted(spec["lod_tris"], reverse=True)
    assert classes["house"]["lod_tris"] == [8000, 2500, 600]
    assert classes["major_building"]["tex"] == 2048


def test_build_job_reads_budgets() -> None:
    """The job carries budgets, texture size and category from the data file."""
    job = _job()
    assert job["lod_tris"] == [8000, 2500, 600]
    assert job["tex"] == 1024
    assert job["out_dir"].endswith("dn/buildings")
    assert job["axis"] == "length"
    assert _job(asset_class="major_building")["tex"] == 2048
    assert _job(tex=512)["tex"] == 512


@pytest.mark.parametrize(
    "overrides",
    [
        {"asset_class": "dragon"},
        {"length": None},
        {"height": 3.0},
        {"length": -1.0},
        {"lods": 4},
        {"asset_id": "Bad-Id"},
    ],
)
def test_build_job_rejects_bad_requests(overrides) -> None:
    """Unknown class, zero or two sizes, bad LOD count or id are refused."""
    with pytest.raises(dn_ingest.IngestError):
        _job(**overrides)


def test_check_result_flags_overrun_and_saturation() -> None:
    """Over-budget LODs and a saturated albedo are reported; under-budget is fine."""
    classes = dn_ingest.load_classes()
    job = _job()
    good = {"triangles": [8000, 2000, 500], "grade": {"after": {"p95_s": 0.4}}}
    assert dn_ingest.check_result(job, good, classes) == []
    bad = {"triangles": [9000, 2000, 500], "grade": {"after": {"p95_s": 0.6}}}
    assert len(dn_ingest.check_result(job, bad, classes)) == 2


def test_parse_result_and_manifest_roundtrip(tmp_path: Path) -> None:
    """DN_RESULT is parsed; manifest entries are inserted, replaced and schema-valid."""
    result = dn_ingest.parse_result(
        "noise\nDN_RESULT "
        + json.dumps(
            {
                "dimensions_m": {"length": 8.0, "width": 5.0, "height": 5.5},
                "triangles": [8000, 2500, 600],
                "grade": None,
            }
        )
    )
    with pytest.raises(dn_ingest.IngestError):
        dn_ingest.parse_result("nothing")
    job = _job(out_dir=dn_ingest.OUT_ROOT)
    entry = dn_ingest.manifest_entry(
        job, result, "house", model_3d="trellis", cost_usd=0.0
    )
    assert entry["files"][0] == "dn/buildings/house_a_lod0.glb"
    manifest = tmp_path / "m.json"
    manifest.write_text('{"assets": {}}', encoding="utf-8")
    dn_ingest.update_manifest(entry, manifest)
    dn_ingest.update_manifest({**entry, "tex": 512}, manifest)
    stored = json.loads(manifest.read_text(encoding="utf-8"))
    assert (
        list(stored["assets"]) == ["house_a"]
        and stored["assets"]["house_a"]["tex"] == 512
    )
    assert_matches_schema(manifest, "art_dn_manifest.schema.json")


def test_hsv_roundtrip() -> None:
    """RGB -> HSV -> RGB is the identity."""
    rgb = np.random.default_rng(1).random((500, 3))
    assert np.allclose(dn_grade.hsv_to_rgb(dn_grade.rgb_to_hsv(rgb)), rgb, atol=1e-9)


def test_grade_caps_saturation_and_lifts_luma() -> None:
    """Saturation is capped at 0.40; a dark albedo is lifted toward the luma window."""
    rgb = np.random.default_rng(2).random((2000, 3)) * 0.3
    rgb[:, 0] += 0.1
    graded, report = dn_grade.grade_albedo(rgb, 0.40, (0.18, 0.35))
    assert dn_grade.rgb_to_hsv(graded)[..., 1].max() <= 0.40 + 1e-9
    assert report["after"]["mean_luma"] >= report["before"]["mean_luma"]
    assert 0.17 <= report["after"]["mean_luma"] <= 0.36
    grey = np.full((10, 3), 0.25)
    assert np.allclose(dn_grade.grade_albedo(grey, 0.4, (0.18, 0.35))[0], grey)


def test_auto_gamma_reaches_window_and_keeps_inside() -> None:
    """Automatic gamma lifts a dark albedo into the window, a bright one down, and leaves a good one."""
    window = (0.18, 0.35)
    rng = np.random.default_rng(3)
    for scale in (0.15, 1.0):
        rgb = rng.random((3000, 3)) * scale
        graded, report = dn_grade.grade_albedo(rgb, 0.40, window)
        assert window[0] <= report["after"]["mean_luma"] <= window[1], scale
        assert dn_grade.rgb_to_hsv(graded)[..., 1].max() <= 0.40 + 1e-9
    inside = np.full((10, 3), 0.25)
    assert dn_grade.auto_gamma(inside, window) == 1.0


def test_job_carries_surface_and_cleanup_settings() -> None:
    """Roughness, base stripping and island threshold come from the class table."""
    job = _job()
    assert (
        job["roughness"] == 0.9
        and job["strip_base"] is True
        and job["island_min"] == 0.01
    )
    assert _job(asset_class="rock")["strip_base"] is False
    assert _job()["gamma"] is None
