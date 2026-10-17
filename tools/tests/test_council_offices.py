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
