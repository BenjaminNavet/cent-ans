"""Validates the stylised town maquette table against its schema (lot GC2, ADR 0158)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"
PROVINCE_FIELDS = {"cultures": "culture", "regions": "region", "religions": "religion"}


def _document() -> dict:
    return json.loads(
        (DATA / "art" / "town_maquettes.json").read_text(encoding="utf-8")
    )


def _province_values(field: str) -> set[str]:
    return {
        json.loads(path.read_text(encoding="utf-8")).get(field, "")
        for path in (DATA / "provinces").glob("*.json")
    }


def test_family_lists_cover_provinces_once() -> None:
    """Every culture, region and religion of a province is in exactly one family."""
    document = _document()
    assert document["default_family"] in document["families"]
    for list_name, field in PROVINCE_FIELDS.items():
        listed: list[str] = []
        for family in document["families"].values():
            listed.extend(family[list_name])
        assert len(listed) == len(set(listed)), list_name
        assert set(listed) == _province_values(field), list_name


def test_expected_families() -> None:
    """A few historical anchors around 1340."""
    families = _document()["families"]
    assert "cul_tuscan" in families["med"]["cultures"]
    assert "cul_greek" in families["byz"]["cultures"]
    assert "cul_novgorodian" in families["rus"]["cultures"]
    assert "cul_maghrebi" in families["isl"]["cultures"]
    assert "cul_kipchak" in families["steppe"]["cultures"]
    assert "cul_french" in families["west"]["cultures"]
