"""Tests for the budget ledger (parsing, rounding, cap check)."""

from decimal import Decimal
from pathlib import Path

from cent_ans_tools import budget
from cent_ans_tools.budget import BudgetLedger, format_amount, parse_amount, to_money


def test_parse_amount_french_and_english_formats() -> None:
    """Both comma and dot decimals parse to the same Decimal."""
    assert parse_amount("12,34 $") == Decimal("12.34")
    assert parse_amount("12.34 $") == Decimal("12.34")
    assert parse_amount("0,00 $") == Decimal("0.00")


def test_to_money_rounds_half_up_to_cents() -> None:
    """Rounding is half-up on cents and immune to float noise."""
    assert to_money(0.005) == Decimal("0.01")
    assert to_money("0.004") == Decimal("0.00")
    assert to_money(0.1 + 0.2) == Decimal("0.30")
    assert format_amount(Decimal("1.5")) == "1,50 $"


def test_initial_file_parses_placeholder(budget_file: Path) -> None:
    """The shipped file has one placeholder row and a zero total."""
    ledger = BudgetLedger(budget_file)
    assert len(ledger.entries) == 1
    assert ledger.entries[0].is_placeholder
    assert ledger.total() == Decimal("0.00")
    assert "Plafond" in "\n".join(ledger.header_lines)


def test_add_entry_replaces_placeholder_and_recomputes_cumul(budget_file: Path) -> None:
    """Rows append, cumul is recomputed from real costs, file keeps its format."""
    ledger = BudgetLedger(budget_file)
    ledger.add_entry("2026-09-23", "OpenRouter", "portrait test", 0.04, 0.035)
    ledger.add_entry("2026-09-24", "OpenRouter", "icône", "0.10", Decimal("0.125"))

    reloaded = BudgetLedger(budget_file)
    assert [entry.service for entry in reloaded.entries] == ["OpenRouter", "OpenRouter"]
    assert reloaded.entries[0].cumulative == Decimal("0.04")  # 0.035 rounded half-up
    assert reloaded.entries[1].actual == Decimal("0.13")
    assert reloaded.total() == Decimal("0.17")

    text = budget_file.read_text(encoding="utf-8")
    assert text.startswith("# Budget cloud (v1)")
    assert "| Date | Service | Objet | Coût estimé | Coût réel | Cumul |" in text
    assert "| 2026-09-24 | OpenRouter | icône | 0,10 $ | 0,13 $ | 0,17 $ |" in text
    assert "Aucune dépense" not in text


def test_check_against_cap(budget_file: Path) -> None:
    """check() is inclusive of the cap and accounts for the recorded cumul."""
    ledger = BudgetLedger(budget_file)
    assert ledger.check(50)
    assert not ledger.check("50.01")
    ledger.add_entry("2026-09-23", "OpenRouter", "test", 49, 49)
    assert ledger.check(1)
    assert not ledger.check(1.01)


def test_module_level_shortcuts(budget_file: Path) -> None:
    """Module functions operate on the given path."""
    budget.add_entry("2026-09-23", "OpenRouter", "test", 1, 2, path=budget_file)
    assert budget.total(budget_file) == Decimal("2.00")
    assert budget.check(48, path=budget_file)
    assert not budget.check(48.01, path=budget_file)


def test_multi_session_file_reads_every_table(multi_session_budget_file: Path) -> None:
    """A file with a table per session (docs/budget.md since session 7) parses both tables.

    Each session keeps its own cumulative column; the untitled first table is "principal".
    """
    ledger = BudgetLedger(multi_session_budget_file)
    assert [session.title for session in ledger.sessions] == [
        None,
        "Session 7 (nuit du 24/09) — enveloppe propre de 50 $",
    ]
    assert len(ledger.sessions[0].entries) == 1
    assert len(ledger.sessions[1].entries) == 1
    assert ledger.entries[0].actual == Decimal("19.20")  # flattened, in file order
    assert ledger.entries[1].actual == Decimal("0.64")
    assert ledger.session_totals() == {
        "principal": Decimal("19.20"),
        "Session 7 (nuit du 24/09) — enveloppe propre de 50 $": Decimal("0.64"),
    }


def test_multi_session_total_and_check_use_the_current_session_only(
    multi_session_budget_file: Path,
) -> None:
    """The cap check is per session: each session has its own clean envelope.

    The first table's 19,20 $ must not count against session 7's.
    """
    ledger = BudgetLedger(multi_session_budget_file)
    assert ledger.total() == Decimal("0.64")
    assert ledger.check(49.36)
    assert not ledger.check(49.37)


def test_multi_session_add_entry_writes_into_the_current_session(
    multi_session_budget_file: Path,
) -> None:
    """New spending is appended to the last table (session 7).

    The first table, its heading and its own cumulative column are left untouched.
    """
    ledger = BudgetLedger(multi_session_budget_file)
    ledger.add_entry("2026-09-25", "OpenRouter", "carte UR2", 0.10, 0.11)

    reloaded = BudgetLedger(multi_session_budget_file)
    assert len(reloaded.sessions[0].entries) == 1
    assert reloaded.sessions[0].entries[0].cumulative == Decimal("19.20")
    assert [entry.subject for entry in reloaded.sessions[1].entries] == [
        "UR1 : illustrations",
        "carte UR2",
    ]
    assert reloaded.sessions[1].entries[-1].cumulative == Decimal("0.75")
    assert reloaded.total() == Decimal("0.75")

    text = multi_session_budget_file.read_text(encoding="utf-8")
    assert "## Session 7 (nuit du 24/09) — enveloppe propre de 50 $" in text
    assert "| Cumul session 7 |" in text
    assert (
        "| 2026-09-24 | OpenRouter | portraits | 19,00 $ | 19,20 $ | 19,20 $ |" in text
    )
    assert "| 2026-09-25 | OpenRouter | carte UR2 | 0,10 $ | 0,11 $ | 0,75 $ |" in text


def test_multi_session_file_round_trips_byte_identical(
    multi_session_budget_file: Path,
) -> None:
    """Parsing and re-rendering without changes reproduces the file exactly."""
    before = multi_session_budget_file.read_text(encoding="utf-8")
    BudgetLedger(multi_session_budget_file).save()
    assert multi_session_budget_file.read_text(encoding="utf-8") == before


def test_trailing_text_after_last_table_is_preserved(tmp_path: Path) -> None:
    """A closing section with no table (after the last one) survives load/save."""
    path = tmp_path / "budget.md"
    path.write_text(
        "# Budget\n"
        "\n"
        "| Date | Service | Objet | Coût estimé | Coût réel | Cumul |\n"
        "|---|---|---|---|---|---|\n"
        "| 2026-09-23 | OpenRouter | test | 1,00 $ | 1,00 $ | 1,00 $ |\n"
        "\n"
        "## Notes de clôture\n"
        "\n"
        "Session terminée, plafond respecté.\n",
        encoding="utf-8",
    )
    ledger = BudgetLedger(path)
    assert "Notes de clôture" in ledger.trailing
    ledger.save()
    text = path.read_text(encoding="utf-8")
    assert "## Notes de clôture" in text
    assert "Session terminée, plafond respecté." in text

    # A later add_entry() must not lose it either.
    ledger.add_entry("2026-09-24", "OpenRouter", "test 2", 1, 1)
    text = path.read_text(encoding="utf-8")
    assert "## Notes de clôture" in text


def test_real_budget_file_parses_and_round_trips() -> None:
    """The actual docs/budget.md (two tables since session 7) parses cleanly and round-trips."""
    from cent_ans_tools.budget import DEFAULT_BUDGET_PATH

    before = DEFAULT_BUDGET_PATH.read_text(encoding="utf-8")
    ledger = BudgetLedger(DEFAULT_BUDGET_PATH)
    assert len(ledger.sessions) >= 2
    assert ledger.render() == before
