"""Shared fixtures: a pristine budget file (header of docs/budget.md, empty table)."""

from pathlib import Path

import pytest

REPO_BUDGET = Path(__file__).resolve().parents[2] / "docs" / "budget.md"
EMPTY_TABLE = (
    "| Date | Service | Objet | Coût estimé | Coût réel | Cumul |\n"
    "|---|---|---|---|---|---|\n"
    "| — | — | Aucune dépense pour l'instant | 0,00 $ | 0,00 $ | 0,00 $ |\n"
)


@pytest.fixture
def budget_file(tmp_path: Path) -> Path:
    """Budget file with the real header and no spending, so tests never touch the real one.

    The real ledger accumulates entries over time; tests need a known empty state.
    """
    header = REPO_BUDGET.read_text(encoding="utf-8").split("| Date |", 1)[0]
    target = tmp_path / "budget.md"
    target.write_text(header.rstrip("\n") + "\n\n" + EMPTY_TABLE, encoding="utf-8")
    return target
