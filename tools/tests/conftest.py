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


@pytest.fixture
def da_then_po_budget_file(budget_file: Path) -> Path:
    """Shape of the real `docs/budget.md`: a funding session (DA) followed by a later one.

    A later, unrelated session (PO) is opened underneath it, the way `PO0` was appended
    after `DA7c`. Reproduces the RS-H bug: a DA5 tool batch (ink icons) is funded by the DA
    envelope, but `add_entry()` with no explicit session always lands in the *last* table
    (PO here), not the one that actually funds it.
    """
    header = REPO_BUDGET.read_text(encoding="utf-8").split("| Date |", 1)[0]
    da_table = (
        "## Direction artistique (25/09) — plafond propre de 50 $\n"
        "\n"
        "| Date | Service | Objet | Coût estimé | Coût réel | Cumul DA |\n"
        "|---|---|---|---|---|---|\n"
        "| 2026-09-26 | OpenRouter | DA7c : icônes de trait à l'encre | 2,69 $ | 2,68 $ | 20,56 $ |\n"
    )
    po_table = (
        "## Polish PO (27/09) — 0 $ prévu, enveloppe ≤ 3 $\n"
        "\n"
        "| Date | Service | Objet | Coût estimé | Coût réel | Cumul PO |\n"
        "|---|---|---|---|---|---|\n"
        "| 2026-09-27 | — | PO0 : planche « avant », gabarit, squelette | 0,00 $ | 0,00 $ | 0,00 $ |\n"
    )
    target = budget_file.parent / "da_then_po_budget.md"
    target.write_text(
        header.rstrip("\n") + "\n\n" + da_table + "\n" + po_table,
        encoding="utf-8",
    )
    return target
