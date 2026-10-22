"""Les `neighbors` des fichiers de provinces suivent l'adjacence terrestre de la géométrie (lot B3)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def _geometry_neighbors() -> dict[str, list[str]]:
    collection = json.loads((DATA / "map" / "provinces.geojson").read_text("utf-8"))
    return {
        feature["properties"]["id"]: feature["properties"]["neighbors"]
        for feature in collection["features"]
    }


def _files() -> dict[str, dict]:
    return {
        path.stem: json.loads(path.read_text("utf-8"))
        for path in sorted((DATA / "provinces").glob("*.json"))
    }


def test_province_files_match_the_geometry_graph() -> None:
    """`neighbors` d'un fichier source = voisins terrestres de `provinces.geojson`."""
    geometry = _geometry_neighbors()
    for pid, province in _files().items():
        assert sorted(province["neighbors"]) == sorted(geometry[pid]), pid


def test_neighbors_are_symmetric() -> None:
    """Si A voisine B, B voisine A."""
    files = _files()
    for pid, province in files.items():
        for other in province["neighbors"]:
            assert pid in files[other]["neighbors"], (pid, other)


def test_guyenne_is_split_in_three_english_provinces() -> None:
    """Bordelais, Bazadais et Gascogne sont trois provinces anglaises contiguës (ADR 0730)."""
    files = _files()
    trio = ("prov_guyenne", "prov_bazadais", "prov_gascogne")
    for pid in trio:
        assert files[pid]["owner"] == "fac_england", pid
        assert set(trio) - {pid} <= set(files[pid]["neighbors"]), pid
