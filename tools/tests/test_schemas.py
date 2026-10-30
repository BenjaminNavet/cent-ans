"""Every data file is valid against its JSON schema, through the explicit manifest.

`data/schemas/manifest.json` maps data globs to schemas (ADR 0829). Three guards:
every JSON/YAML file of `data/` must match an entry or a justified exception, every
entry must match at least one file, every schema must be used.
"""

import json
import subprocess
from pathlib import Path

import pytest
from conftest import DATA, assert_matches_schema

MANIFEST = json.loads((DATA / "schemas" / "manifest.json").read_text(encoding="utf-8"))
DATA_SUFFIXES = {".json", ".yaml", ".yml"}

# Schémas sans données propres (modules partagés de $ref), avec la raison.
EXEMPT_SCHEMAS: dict[str, str] = {
    "common.schema.json": "définitions partagées (`$defs`), aucun document de données",
    "battle_replay.schema.json": "échantillon écrit par sim-battle, validé par test_battle_replay_schema.py",
}


def _ignored_by_git() -> set[Path]:
    """Generated data files that git ignores (e.g. `data/map/real/map.json`)."""
    listing = subprocess.run(
        ["git", "ls-files", "--others", "--ignored", "--exclude-standard", "--", "."],
        cwd=DATA,
        capture_output=True,
        text=True,
        check=False,
    ).stdout
    return {Path(line) for line in listing.splitlines()}


def _data_files() -> list[Path]:
    """Every tracked JSON/YAML data file, except the schemas themselves."""
    schema_dir = DATA / "schemas"
    ignored = _ignored_by_git()
    return sorted(
        path
        for path in DATA.rglob("*")
        if path.is_file()
        and path.relative_to(DATA) not in ignored
        and path.suffix in DATA_SUFFIXES
        and schema_dir not in path.parents
    )


def _relative(path: Path) -> str:
    return path.relative_to(DATA).as_posix()


def _cases() -> list:
    cases = []
    for entry in MANIFEST["entries"]:
        paths = sorted(DATA.glob(entry["glob"]))
        cases.extend(
            pytest.param(entry["schema"], path, id=_relative(path)) for path in paths
        )
    return cases


@pytest.mark.parametrize(("schema_name", "data_path"), _cases())
def test_data_matches_schema(schema_name: str, data_path: Path) -> None:
    assert_matches_schema(data_path, schema_name)


def test_every_data_file_has_a_schema_or_an_exception() -> None:
    covered = {
        _relative(path)
        for entry in MANIFEST["entries"]
        for path in DATA.glob(entry["glob"])
    }
    excepted = {
        _relative(path)
        for exception in MANIFEST["exceptions"]
        for path in DATA.glob(exception["glob"])
    }
    unmatched = [
        _relative(path)
        for path in _data_files()
        if _relative(path) not in covered | excepted
    ]
    assert not unmatched, f"{len(unmatched)} fichiers sans schéma : {unmatched[:20]}"


def test_every_manifest_entry_matches_a_file() -> None:
    dead = [
        entry["glob"]
        for entry in [*MANIFEST["entries"], *MANIFEST["exceptions"]]
        if not list(DATA.glob(entry["glob"]))
    ]
    assert not dead, f"entrées du manifeste sans fichier : {dead}"


def test_exceptions_are_justified() -> None:
    bare = [e["glob"] for e in MANIFEST["exceptions"] if len(e.get("reason", "")) < 20]
    assert not bare, f"exceptions sans raison : {bare}"


def test_every_schema_is_validated_or_exempt() -> None:
    schema_files = {path.name for path in (DATA / "schemas").glob("*.schema.json")}
    used = {entry["schema"] for entry in MANIFEST["entries"]}
    uncovered = sorted(schema_files - used - set(EXEMPT_SCHEMAS))
    assert not uncovered, f"schémas sans validation de données : {uncovered}"
    assert not sorted(used - schema_files), "entrée du manifeste sans schéma"
