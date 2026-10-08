"""Validates data/fx/animal_motion.json (lots AS1, AS8c) and its link to the measured values."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_animal_motion_matches_schema() -> None:
    """The motion file matches its schema."""
    schema = _load("schemas/fx_animal_motion.schema.json")
    Draft202012Validator.check_schema(schema)
    errors = list(
        Draft202012Validator(schema).iter_errors(_load("fx/animal_motion.json"))
    )
    assert not errors, [error.message for error in errors]


def test_gait_curves_come_from_the_measurement() -> None:
    """The look-up tables in the settings are the ones the measurement script produced."""
    motion = _load("fx/animal_motion.json")
    measured = _load(motion["source"]["measured_file"])
    defaults = motion["campaign"]["defaults"]
    assert defaults["swing_lut"] == measured["cattle"]["swing_lut"]
    assert defaults["lift_lut"] == measured["cattle"]["lift_lut"]
    lines = measured["wagon"]["jolt_lines"]
    stride = measured["wagon"]["horse_stride_m"]
    for setting, line in zip(defaults["jolt_lines"], lines, strict=True):
        assert abs(setting["amp_m"] - line["amp_m"]) < 1e-4
        assert abs(setting["stride_ratio"] * stride - line["spatial_period_m"]) < 0.02
    camp = motion["camp_horse"]
    assert abs(camp["chew_hz"] - measured["horse_rest"]["chew_peaks_hz"][0][0]) < 0.05
    assert abs(camp["head_lift_rad"] - measured["horse_rest"]["head_lift_rad"]) < 0.01


def test_lut_phase_origin() -> None:
    """The swing table starts at the mid-swing instant (zero, rising) and is normalised."""
    swing = _load("fx/animal_motion.json")["campaign"]["defaults"]["swing_lut"]
    assert abs(swing[0]) < 0.05 and swing[1] > swing[0]
    assert 0.9 <= max(abs(v) for v in swing) <= 1.1
    lift = _load("fx/animal_motion.json")["campaign"]["defaults"]["lift_lut"]
    assert min(lift) == 0.0 and max(lift) == 1.0
