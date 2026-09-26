"""Ledger over ``docs/budget.md``: parse, append, and check the cloud budget cap.

The markdown file can hold several tables, each with the columns ``Date | Service | Objet |
Coût estimé | Coût réel | Cumul`` (the last header cell's text is free, e.g. ``Cumul session
7``); amounts are written in French style (``12,34 $``). A table may be preceded by a ``##``
heading that names its session (e.g. ``## Session 7 (nuit du 24/09) — enveloppe propre de 50
$``) and opens a fresh cumulative column and, per the design doc, its own spending envelope;
the very first table (before any ``##`` heading) has no title. Money is handled with
``Decimal`` to avoid float drift.

``check()``/``add_entry()``/``total()`` all operate on the *current* session, i.e. the last
table in the file: new spending is recorded there, and the cap is checked against its own
cumulative column, not the grand total of every past session. Use ``session_totals()`` for a
per-session breakdown.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from decimal import ROUND_HALF_UP, Decimal
from pathlib import Path

BUDGET_CAP = Decimal("50.00")
DEFAULT_BUDGET_PATH = Path(__file__).resolve().parents[2] / "docs" / "budget.md"

_CENTS = Decimal("0.01")
_AMOUNT_RE = re.compile(r"-?\d+(?:[.,]\d+)?")
_PLACEHOLDER_SERVICE = "—"
_DEFAULT_CUMUL_LABEL = "Cumul"


class BudgetExceeded(RuntimeError):
    """Raised when a paid call would push the cumulative spend above the cap."""


@dataclass
class BudgetEntry:
    """One row of a budget table."""

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


@dataclass
class BudgetSession:
    """One table of the ledger.

    ``title`` is ``None`` for the untitled first table; ``preamble`` is the raw text that
    precedes it, preserved verbatim on render.
    """

    title: str | None
    preamble: str
    cumul_label: str = _DEFAULT_CUMUL_LABEL
    entries: list[BudgetEntry] = field(default_factory=list)

    @property
    def label(self) -> str:
        """Display key for `session_totals()`: the heading, or "principal" for the first table."""
        return self.title if self.title else "principal"


def to_money(value: Decimal | float | int | str) -> Decimal:
    """Convert any numeric input to a Decimal rounded to cents (half-up)."""
    return Decimal(str(value)).quantize(_CENTS, rounding=ROUND_HALF_UP)


def parse_amount(cell: str) -> Decimal:
    """Parse a table cell such as ``"12,34 $"`` into ``Decimal("12.34")``."""
    match = _AMOUNT_RE.search(cell.replace(" ", " "))
    if match is None:
        raise ValueError(f"Montant illisible : {cell!r}")
    return to_money(match.group(0).replace(",", "."))


def format_amount(amount: Decimal) -> str:
    """Format a Decimal as the French-style cell used in the table (``12,34 $``)."""
    return f"{to_money(amount):.2f}".replace(".", ",") + " $"


def _is_table_row(line: str) -> bool:
    return line.strip().startswith("|")


def _split_row(line: str) -> list[str]:
    return [cell.strip() for cell in line.strip().strip("|").split("|")]


def _is_separator_row(cells: list[str]) -> bool:
    return all(set(cell) <= set("-: ") for cell in cells)


class BudgetLedger:
    """Read/write access to the budget markdown file (one or more tables)."""

    def __init__(
        self, path: Path | str = DEFAULT_BUDGET_PATH, cap: Decimal = BUDGET_CAP
    ):
        """Load the ledger from ``path`` with the given spending cap."""
        self.path = Path(path)
        self.cap = cap
        self.sessions: list[BudgetSession] = []
        self.trailing = ""
        self._load()

    def _load(self) -> None:
        lines = self.path.read_text(encoding="utf-8").splitlines()
        raw_sections: list[tuple[list[str], list[str]]] = []
        preamble: list[str] = []
        table: list[str] = []
        in_table = False
        for line in lines:
            if _is_table_row(line):
                in_table = True
                table.append(line)
                continue
            if in_table:
                raw_sections.append((preamble, table))
                preamble, table = [], []
                in_table = False
            preamble.append(line)
        if in_table:
            raw_sections.append((preamble, table))
        else:
            # Text after the last table (e.g. a closing note or a new, table-less
            # section) has no table to attach to: keep it verbatim and re-emit it
            # in render(), instead of silently dropping it on the next save().
            self.trailing = "\n".join(preamble)
        if not raw_sections:
            raise ValueError(f"Aucun tableau de budget trouvé dans {self.path}")
        self.sessions = [
            self._parse_section(preamble_lines, table_lines)
            for preamble_lines, table_lines in raw_sections
        ]

    @staticmethod
    def _parse_section(
        preamble_lines: list[str], table_lines: list[str]
    ) -> BudgetSession:
        title = next(
            (
                stripped[3:].strip()
                for line in preamble_lines
                if (stripped := line.strip()).startswith("## ")
            ),
            None,
        )
        cumul_label = _DEFAULT_CUMUL_LABEL
        entries: list[BudgetEntry] = []
        header_seen = False
        for line in table_lines:
            cells = _split_row(line)
            if not header_seen:
                header_seen = True
                if len(cells) == 6:
                    cumul_label = cells[5]
                continue
            if _is_separator_row(cells):
                continue
            entries.append(BudgetLedger._parse_row(cells))
        return BudgetSession(
            title=title,
            preamble="\n".join(preamble_lines),
            cumul_label=cumul_label,
            entries=entries,
        )

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

    @staticmethod
    def _recompute(session: BudgetSession) -> None:
        running = Decimal("0.00")
        for entry in session.entries:
            running = to_money(running + entry.actual)
            entry.cumulative = running

    @property
    def entries(self) -> list[BudgetEntry]:
        """Every row of every session, in file order (read-only, backward compatibility)."""
        return [entry for session in self.sessions for entry in session.entries]

    @property
    def header_lines(self) -> list[str]:
        """Lines of the document header, before the first table (backward compatibility)."""
        return self.sessions[0].preamble.splitlines()

    @property
    def current_session(self) -> BudgetSession:
        """The last table in the file: where new spending is recorded and the cap is checked."""
        return self.sessions[-1]

    def session_totals(self) -> dict[str, Decimal]:
        """Cumulative real spend of each session, keyed by its heading.

        The untitled first table's key is ``"principal"``.
        """
        return {
            session.label: (
                session.entries[-1].cumulative if session.entries else Decimal("0.00")
            )
            for session in self.sessions
        }

    def total(self) -> Decimal:
        """Cumulative real spend of the current session (the last table in the file)."""
        session = self.current_session
        return session.entries[-1].cumulative if session.entries else Decimal("0.00")

    def check(self, estimated: Decimal | float | int | str) -> bool:
        """Return True if the current session's ``total() + estimated`` stays within the cap."""
        return self.total() + to_money(estimated) <= self.cap

    def add_entry(
        self,
        date: str,
        service: str,
        subject: str,
        estimated: Decimal | float | int | str,
        actual: Decimal | float | int | str,
    ) -> BudgetEntry:
        """Append a row to the current session.

        Drops its placeholder if present, recomputes that session's cumul, and saves.
        """
        session = self.current_session
        entry = BudgetEntry(
            date=date,
            service=service,
            subject=subject,
            estimated=to_money(estimated),
            actual=to_money(actual),
        )
        session.entries = [
            existing for existing in session.entries if not existing.is_placeholder
        ]
        session.entries.append(entry)
        self._recompute(session)
        self.save()
        return entry

    def _render_table(self, session: BudgetSession) -> str:
        rows = [
            f"| Date | Service | Objet | Coût estimé | Coût réel | {session.cumul_label} |",
            "|---|---|---|---|---|---|",
        ]
        for entry in session.entries:
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
        return "\n".join(rows) + "\n"

    def render(self) -> str:
        """Render the whole markdown file (every session's preamble and table, in order).

        Each preamble (the raw text between the previous table, or the start of the file, and
        this one) is stripped of trailing blank lines and reattached to its table with exactly
        one blank line, same as the single-table file always looked.
        """
        rendered = "".join(
            session.preamble.rstrip("\n") + "\n\n" + self._render_table(session)
            for session in self.sessions
        )
        trailing = self.trailing.strip("\n")
        if trailing:
            rendered = rendered.rstrip("\n") + "\n\n" + trailing + "\n"
        return rendered

    def save(self) -> None:
        """Write the ledger back to disk."""
        self.path.write_text(self.render(), encoding="utf-8")


def total(path: Path | str = DEFAULT_BUDGET_PATH) -> Decimal:
    """Module-level shortcut: cumulative spend of the current session in ``path``."""
    return BudgetLedger(path).total()


def session_totals(path: Path | str = DEFAULT_BUDGET_PATH) -> dict[str, Decimal]:
    """Module-level shortcut: cumulative spend of every session in ``path``."""
    return BudgetLedger(path).session_totals()


def check(
    estimated: Decimal | float | int | str, path: Path | str = DEFAULT_BUDGET_PATH
) -> bool:
    """Module-level shortcut: whether ``estimated`` fits under the cap in the current session."""
    return BudgetLedger(path).check(estimated)


def add_entry(
    date: str,
    service: str,
    subject: str,
    estimated: Decimal | float | int | str,
    actual: Decimal | float | int | str,
    path: Path | str = DEFAULT_BUDGET_PATH,
) -> BudgetEntry:
    """Module-level shortcut: append a row to the current session in ``path``."""
    return BudgetLedger(path).add_entry(date, service, subject, estimated, actual)
