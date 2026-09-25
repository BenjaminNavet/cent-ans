"""Validates the NV2 naval data: ship names per faction and port, port seas."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"
NAVAL = DATA / "naval"


def _schema(name: str) -> Draft202012Validator:
    schema = json.loads((DATA / "schemas" / name).read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    return Draft202012Validator(schema)


def _ports() -> dict[str, dict]:
    ports = {}
    for path in (DATA / "settlements").glob("prov_*.json"):
        for settlement in json.loads(path.read_text(encoding="utf-8")):
            if settlement.get("port"):
                ports[settlement["id"]] = settlement
    return ports


def test_ship_names_match_schema_and_reference_ports() -> None:
    """Ship names use known factions and port settlements, never twice a faction."""
    names = json.loads((NAVAL / "ship_names.json").read_text(encoding="utf-8"))
    errors = [
        e.message for e in _schema("naval_ship_names.schema.json").iter_errors(names)
    ]
    assert not errors
    factions = {p.stem for p in (DATA / "factions").glob("*.json")}
    ports = _ports()
    for entry in names["factions"]:
        seen: set[str] = set()
        assert entry["faction"] in factions
        for port in entry.get("ports", {}):
            assert port in ports, port
        every = entry["names"] + [
            n for ns in entry.get("ports", {}).values() for n in ns
        ]
        for name in every:
            assert name not in seen, name
            seen.add(name)


def test_strait_ports_open_onto_the_channel() -> None:
    """Calais, Wissant, Boulogne and Dover open onto the Channel (Pas de Calais)."""
    ports = _ports()
    for port in ("set_calais", "set_wissant", "set_boulogne", "set_dover"):
        assert ports[port].get("sea_zone") == "sea_channel", port
    fleets = json.loads((NAVAL / "fleets.json").read_text(encoding="utf-8"))
    for port in fleets.get("port_waters", {}):
        assert port in ports, port
