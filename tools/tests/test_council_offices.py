"""Checks data/council/offices.json beyond the schema (ADR 0341, lot RG13a)."""

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"
UI = ROOT / "game" / "scripts" / "ui"


def _offices() -> list[dict]:
    return json.loads((DATA / "council" / "offices.json").read_text(encoding="utf-8"))[
        "offices"
    ]


def _block_keys(source: str, constant: str) -> set[str]:
    """Top-level string keys of the `const <constant> := { ... }` dictionary."""
    start = source.index(f"const {constant}")
    body = source[source.index("{", start) :]
    depth, end = 0, 0
    for index, char in enumerate(body):
        depth += char == "{"
        depth -= char == "}"
        if depth == 0:
            end = index
            break
    keys = set()
    depth = 0
    for line in body[: end + 1].splitlines():
        if depth == 1:
            match = re.match(r'\s*"([a-z_]+)"\s*:', line)
            if match:
                keys.add(match.group(1))
        depth += line.count("{") - line.count("}")
    return keys


def test_five_offices_with_unique_ids_and_kinds() -> None:
    """Five offices; every advice kind belongs to exactly one of them."""
    offices = _offices()
    assert [o["id"] for o in offices] == [
        "chancellor",
        "constable",
        "treasurer",
        "chaplain",
        "chamberlain",
    ]
    seen: dict[str, str] = {}
    for office in offices:
        for kind in office["advice_kinds"]:
            assert kind not in seen, f"{kind}: {seen[kind]} and {office['id']}"
            seen[kind] = office["id"]


def test_every_existing_alert_and_hint_kind_is_mapped() -> None:
    """Each alert, hint or journal kind the UI already produces has an office."""
    mapped = {kind for office in _offices() for kind in office["advice_kinds"]}
    cluster = (UI / "end_turn_cluster.gd").read_text(encoding="utf-8")
    report = (UI / "season_report.gd").read_text(encoding="utf-8")
    alerts = (UI / "alerts.gd").read_text(encoding="utf-8")
    known = (
        _block_keys(cluster, "SHORT_LABELS")
        | _block_keys(cluster, "KIND_LABELS")
        | _block_keys(report, "KIND_STYLES")
        | set(re.findall(r'"kind": "([a-z_]+)"', alerts))
    )
    hints = json.loads((DATA / "ui" / "next_hints.json").read_text(encoding="utf-8"))
    known |= {hint["id"] for hint in hints["hints"]}
    assert len(known) > 20
    assert known - mapped == set(), f"unmapped kinds: {sorted(known - mapped)}"
