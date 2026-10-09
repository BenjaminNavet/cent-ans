"""Player rules in ``tools/experiments/dn_batch.py``: no TRELLIS 2, no paid best-of-N."""

import importlib.util
import sys
from pathlib import Path

import pytest

SCRIPT = Path(__file__).resolve().parents[1] / "experiments" / "dn_batch.py"


@pytest.fixture(scope="module")
def batch():
    """The dn_batch module."""
    spec = importlib.util.spec_from_file_location("dn_batch_options_under_test", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_fal2_backend_refused(batch):
    """TRELLIS 2 is rejected from the CLI override and from a catalogue entry."""
    with pytest.raises(ValueError, match="forbidden"):
        batch.validate_options("fal2", 0, "local")
    with pytest.raises(ValueError, match="forbidden"):
        batch.validate_options("", 0, "local", [{"id": "x", "backend3d": "fal2"}])
    with pytest.raises(ValueError, match="forbidden"):
        batch.backends({"backend3d": "fal2"})


def test_allowed_backends(batch):
    """TRELLIS 1 (fal), HF, SF3D and combinations pass."""
    for value in ("fal", "sf3d", "hf", "both", "hf+sf3d"):
        batch.validate_options(value, 0, "local")


def test_paid_best_of_n_refused(batch):
    """Several seeds on a paid image model are refused; one is fine."""
    with pytest.raises(ValueError, match="best-of-N"):
        batch.validate_options("fal", 3, "fal")
    with pytest.raises(ValueError, match="best-of-N"):
        batch.validate_options("", 0, "fal", [{"id": "x", "seeds": 3}])
    batch.validate_options("fal", 1, "fal", [{"id": "x", "seeds": 1}])


def test_local_best_of_n_allowed(batch):
    """Several seeds stay allowed with local (free) images."""
    batch.validate_options("fal", 3, "local", [{"id": "x", "seeds": 3}])


def test_local_fallback_off_by_default(batch):
    """No local fallback unless ``--local-fallback`` is given."""
    assert batch.LOCAL_FALLBACK is False
