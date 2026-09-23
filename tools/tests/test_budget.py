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
