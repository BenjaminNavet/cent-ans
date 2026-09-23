"""Shared fixtures: a temporary copy of docs/budget.md."""

import shutil
from pathlib import Path

import pytest

REPO_BUDGET = Path(__file__).resolve().parents[2] / "docs" / "budget.md"


@pytest.fixture
def budget_file(tmp_path: Path) -> Path:
    """Copy the real budget markdown into a temp dir so tests never touch it."""
    target = tmp_path / "budget.md"
    shutil.copy(REPO_BUDGET, target)
    return target
