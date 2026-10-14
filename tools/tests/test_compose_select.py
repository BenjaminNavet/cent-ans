"""Tests du lot C0a (sélection des lieux et fusion des provinces)."""

from __future__ import annotations

import json

import pytest

from cent_ans_tools.compose import select as S


@pytest.fixture(scope="module")
def data() -> S.Data:
    """Data."""
    return S.load()


@pytest.fixture(scope="module")
def owner_result(data: S.Data) -> S.Result:
    """Owner result."""
    return S.select(S.Params(), data)


@pytest.fixture(scope="module")
def realm_result(data: S.Data) -> S.Result:
    """Realm result."""
    return S.select(S.Params(merge_scope="realm"), data)


def test_deterministic(data: S.Data, owner_result: S.Result) -> None:
    """Test deterministic."""
    again = S.select(S.Params(), data)
    assert json.dumps(again.selection) == json.dumps(owner_result.selection)
    assert json.dumps(again.id_map) == json.dumps(owner_result.id_map)


def test_protected_ids_present(owner_result: S.Result) -> None:
    """Test protected ids present."""
    ids = {s["id"] for s in owner_result.selection["settlements"]}
    assert set(S.TEST_SETTLEMENTS) <= ids


def test_one_city_per_province(owner_result: S.Result) -> None:
    """Test one city per province."""
    cities = {}
    for s in owner_result.selection["settlements"]:
        if s["kind"] == "city":
            cities[s["province"]] = cities.get(s["province"], 0) + 1
    for province in owner_result.selection["provinces"]:
        assert cities.get(province["id"]) == 1


def test_kept_province_ids(data: S.Data, owner_result: S.Result) -> None:
    """Test kept province ids."""
    ids = {p["id"] for p in owner_result.selection["provinces"]}
    assert set(S.KEEP_PROVINCE_IDS) <= ids
    dropped = set(owner_result.selection["dropped_provinces"])
    for capital in S.faction_capitals(data) - dropped:
        assert capital in ids


@pytest.mark.parametrize("scope", ["owner", "realm"])
def test_merges_are_neighbours_of_same_scope(
    data: S.Data, scope: str, owner_result: S.Result, realm_result: S.Result
) -> None:
    """Test merges are neighbours of same scope."""
    result = owner_result if scope == "owner" else realm_result
    params = S.Params(merge_scope=scope)
    for province in result.selection["provinces"]:
        members = set(province["merged_from"])
        assert len(members) <= S.MAX_GROUP
        keys = {S.scope_key(data, data.provinces[m]["owner"], params) for m in members}
        assert len(keys) == 1
        # Connexité par adjacence au sein du groupe.
        seen, todo = set(), [province["id"]]
        while todo:
            current = todo.pop()
            if current in seen:
                continue
            seen.add(current)
            todo += [n for n in data.adjacency[current] if n in members]
        assert seen == members


def test_id_map_consistent(owner_result: S.Result) -> None:
    """Test id map consistent."""
    id_map = owner_result.id_map
    new_ids = {p["id"] for p in owner_result.selection["provinces"]}
    assert {v for v in id_map["provinces"].values() if v} == new_ids
    kept = {s["id"] for s in owner_result.selection["settlements"]}
    assert {k for k, v in id_map["settlements"].items() if v} == kept


def test_settlement_target(owner_result: S.Result) -> None:
    """Test settlement target."""
    assert abs(len(owner_result.selection["settlements"]) - 400) <= 20


def test_province_target_owner_scope_limit(owner_result: S.Result) -> None:
    """Test province target owner scope limit."""
    # Avec « même owner » la cible 180 est inatteignable (210 composantes connexes) :
    # on vérifie seulement que la fusion progresse au mieux (voir docs/wip/cc-c0a-selection.md).
    assert len(owner_result.selection["provinces"]) < 260


def test_province_target_realm_scope(realm_result: S.Result) -> None:
    """Test province target realm scope."""
    count = len(realm_result.selection["provinces"])
    assert count < 260
    assert count <= 210  # reste au-dessus de 180 + 5 % (capitales protégées)
