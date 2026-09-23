"""Ledger over ``docs/budget.md``: parse, append, and check the cloud budget cap.

The markdown file holds a single table with the columns
``Date | Service | Objet | Coût estimé | Coût réel | Cumul``; amounts are written
in French style (``12,34 $``). Money is handled with ``Decimal`` to avoid float drift.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from decimal import ROUND_HALF_UP, Decimal
from pathlib import Path

BUDGET_CAP = Decimal("50.00")
DEFAULT_BUDGET_PATH = Path(__file__).resolve().parents[2] / "docs" / "budget.md"

_CENTS = Decimal("0.01")
_AMOUNT_RE = re.compile(r"-?\d+(?:[.,]\d+)?")
_PLACEHOLDER_SERVICE = "—"


class BudgetExceeded(RuntimeError):
    """Raised when a paid call would push the cumulative spend above the cap."""


@dataclass
class BudgetEntry:
    """One row of the budget table."""

    date: str
    service: str
    subject: str
    estimated: Decimal
    actual: Decimal
    cumulative: Decimal = Decimal("0.00")

    @property
    def is_placeholder(self) -> bool:
        """Whether the row is the initial "no spending yet" placeholder."""
        return self.service == _PLACEHOLDER_SERVICE


def to_money(value: Decimal | float | int | str) -> Decimal:
    """Convert any numeric input to a Decimal rounded to cents (half-up)."""
    return Decimal(str(value)).quantize(_CENTS, rounding=ROUND_HALF_UP)


def parse_amount(cell: str) -> Decimal:
    """Parse a table cell such as ``"12,34 $"`` into ``Decimal("12.34")``."""
    match = _AMOUNT_RE.search(cell.replace(" ", " "))
    if match is None:
        raise ValueError(f"Montant illisible : {cell!r}")
    return to_money(match.group(0).replace(",", "."))


def format_amount(amount: Decimal) -> str:
    """Format a Decimal as the French-style cell used in the table (``12,34 $``)."""
    return f"{to_money(amount):.2f}".replace(".", ",") + " $"


class BudgetLedger:
    """Read/write access to the budget markdown table."""

    def __init__(
        self, path: Path | str = DEFAULT_BUDGET_PATH, cap: Decimal = BUDGET_CAP
    ):
        """Load the ledger from ``path`` with the given spending cap."""
        self.path = Path(path)
        self.cap = cap
        self.header_lines: list[str] = []
        self.entries: list[BudgetEntry] = []
        self._load()

    def _load(self) -> None:
        lines = self.path.read_text(encoding="utf-8").splitlines()
        in_table = False
        for line in lines:
            stripped = line.strip()
            if stripped.startswith("|"):
                cells = [cell.strip() for cell in stripped.strip("|").split("|")]
                if not in_table:
                    in_table = True
                    continue  # column header
                if all(set(cell) <= set("-: ") for cell in cells):
                    continue  # separator row
                self.entries.append(self._parse_row(cells))
            elif not in_table:
                self.header_lines.append(line)

    @staticmethod
    def _parse_row(cells: list[str]) -> BudgetEntry:
        if len(cells) != 6:
            raise ValueError(f"Ligne de budget invalide : {cells!r}")
        date, service, subject, estimated, actual, cumulative = cells
        return BudgetEntry(
            date=date,
            service=service,
            subject=subject,
            estimated=parse_amount(estimated),
            actual=parse_amount(actual),
            cumulative=parse_amount(cumulative),
        )

    def _recompute(self) -> None:
        running = Decimal("0.00")
        for entry in self.entries:
            running = to_money(running + entry.actual)
            entry.cumulative = running

    def total(self) -> Decimal:
        """Return the cumulative real spend recorded so far."""
        return self.entries[-1].cumulative if self.entries else Decimal("0.00")

    def check(self, estimated: Decimal | float | int | str) -> bool:
        """Return True if ``total() + estimated`` stays within the cap."""
        return self.total() + to_money(estimated) <= self.cap

    def add_entry(
        self,
        date: str,
        service: str,
        subject: str,
        estimated: Decimal | float | int | str,
        actual: Decimal | float | int | str,
    ) -> BudgetEntry:
        """Append a row, drop the placeholder if present, recompute cumul, and save."""
        entry = BudgetEntry(
            date=date,
            service=service,
            subject=subject,
            estimated=to_money(estimated),
            actual=to_money(actual),
        )
        self.entries = [
            existing for existing in self.entries if not existing.is_placeholder
        ]
        self.entries.append(entry)
        self._recompute()
        self.save()
        return entry

    def render(self) -> str:
        """Render the whole markdown file (header + table)."""
        rows = [
            "| Date | Service | Objet | Coût estimé | Coût réel | Cumul |",
            "|---|---|---|---|---|---|",
        ]
        for entry in self.entries:
            rows.append(
                "| "
                + " | ".join(
                    [
                        entry.date,
                        entry.service,
                        entry.subject,
                        format_amount(entry.estimated),
                        format_amount(entry.actual),
                        format_amount(entry.cumulative),
                    ]
                )
                + " |"
            )
        header = "\n".join(self.header_lines).rstrip("\n")
        return f"{header}\n\n" + "\n".join(rows) + "\n"

    def save(self) -> None:
        """Write the ledger back to disk."""
        self.path.write_text(self.render(), encoding="utf-8")


def total(path: Path | str = DEFAULT_BUDGET_PATH) -> Decimal:
    """Module-level shortcut: cumulative spend recorded in ``path``."""
    return BudgetLedger(path).total()


def check(
    estimated: Decimal | float | int | str, path: Path | str = DEFAULT_BUDGET_PATH
) -> bool:
    """Module-level shortcut: whether ``estimated`` fits under the cap."""
    return BudgetLedger(path).check(estimated)


def add_entry(
    date: str,
    service: str,
    subject: str,
    estimated: Decimal | float | int | str,
    actual: Decimal | float | int | str,
    path: Path | str = DEFAULT_BUDGET_PATH,
) -> BudgetEntry:
    """Module-level shortcut: append a row to ``path``."""
    return BudgetLedger(path).add_entry(date, service, subject, estimated, actual)
