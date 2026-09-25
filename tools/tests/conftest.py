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


@pytest.fixture
def multi_session_budget_file(budget_file: Path) -> Path:
    """Same header as `budget_file`, with a first table already spent to 19,20 $.

    A second, titled session is opened underneath it (shape of `docs/budget.md` since
    session 7).
    """
    header = REPO_BUDGET.read_text(encoding="utf-8").split("| Date |", 1)[0]
    first_table = (
        "| Date | Service | Objet | Coût estimé | Coût réel | Cumul |\n"
        "|---|---|---|---|---|---|\n"
        "| 2026-09-24 | OpenRouter | portraits | 19,00 $ | 19,20 $ | 19,20 $ |\n"
    )
    second_table = (
        "## Session 7 (nuit du 24/09) — enveloppe propre de 50 $\n"
        "\n"
        "| Date | Service | Objet | Coût estimé | Coût réel | Cumul session 7 |\n"
        "|---|---|---|---|---|---|\n"
        "| 2026-09-25 | OpenRouter | UR1 : illustrations | 0,64 $ | 0,64 $ | 0,64 $ |\n"
    )
    target = budget_file.parent / "multi_session_budget.md"
    target.write_text(
        header.rstrip("\n") + "\n\n" + first_table + "\n" + second_table,
        encoding="utf-8",
    )
    return target
