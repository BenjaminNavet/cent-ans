"""Validation of the Codex entries (`data/codex/*.json`, H2).

Checks performed by `validate_codex`:

- every entry matches `data/schemas/codex.schema.json` and its id equals its file name;
- every `[[cdx_id]]` / `[[cdx_id|label]]` link, in the codex and in every text field of
  `data/**/*.json`, resolves to an entry or to an id listed in `data/codex/_todo.md` or in another
  pending list `data/codex/_*.md` (links recorded by the diet, medicine and event lots);
- `see_also` ids resolve the same way;
- aliases (and titles) are unique across entries, case-insensitively;
- every `exclude_contexts` expression contains the title or an alias of its entry (B8);
- `[[!text]]` escapes (plain text, never auto-linked) are not links and must not be empty;
- `entity` points to an existing game entity file;
- every `decor` key names an existing natural-decor species and belongs to a single entry (NA).
"""

import json
import re
from collections.abc import Iterator
from dataclasses import dataclass, field
from pathlib import Path

import yaml
from jsonschema import Draft202012Validator
from referencing import Registry, Resource

LINK_PATTERN = re.compile(r"\[\[([^\]|]+)(?:\|[^\]]*)?\]\]")
ESCAPE_PREFIX = "!"
TODO_ID_PATTERN = re.compile(r"`(cdx_[a-z0-9_]+)`")
ENTITY_DIRECTORIES = {
    "chr": "characters",
    "evt": "events",
    "tech": "technologies",
    "bld": "buildings",
    "unit": "unit_types",
    "prov": "provinces",
    "fac": "factions",
    "diet": "diets",
    "res": "resources",
    "edict": "edicts",
    "order": "battle_orders",
    "ability": "battle_abilities",
    "ship": "naval/ships",
}


@dataclass
class CodexReport:
    """Result of a codex validation: entries loaded and problems found."""

    entries: dict[str, dict] = field(default_factory=dict)
    todo_ids: set[str] = field(default_factory=set)
    errors: list[str] = field(default_factory=list)
    link_count: int = 0


def schema_validator(data_dir: Path, schema_name: str) -> Draft202012Validator:
    """Builds a validator for `schema_name` resolving sibling schemas locally."""
    registry = Registry()
    schemas = {}
    for path in (data_dir / "schemas").glob("*.schema.json"):
        schema = json.loads(path.read_text(encoding="utf-8"))
        schemas[path.name] = schema
        resource = Resource.from_contents(schema)
        registry = registry.with_resource(schema["$id"], resource).with_resource(
            path.name, resource
        )
    return Draft202012Validator(schemas[schema_name], registry=registry)


BUNDLE_NAME = "codex_bundle.json"
BUNDLE_VERSION = 1


def build_bundle(data_dir: Path) -> dict:
    """Single-file form of `data_dir/codex/cdx_*.json`, entries sorted by id.

    The game reads this bundle at start-up (one file instead of ~480); the entry files stay the
    editable source of truth. Regenerate with `cent-ans codex-bundle`.
    """
    entries = [
        json.loads(path.read_text(encoding="utf-8"))
        for path in sorted((data_dir / "codex").glob("cdx_*.json"))
    ]
    return {"version": BUNDLE_VERSION, "entries": entries}


def bundle_text(bundle: dict) -> str:
    """Canonical serialisation of a bundle (compact, UTF-8, trailing newline)."""
    return json.dumps(bundle, ensure_ascii=False, separators=(",", ":")) + "\n"


def write_bundle(data_dir: Path) -> Path:
    """Regenerates `data_dir/codex_bundle.json` from the entry files."""
    target = data_dir / BUNDLE_NAME
    target.write_text(bundle_text(build_bundle(data_dir)), encoding="utf-8")
    return target


def iter_strings(value: object, where: str) -> Iterator[tuple[str, str]]:
    """Yields `(location, text)` for every string nested in a JSON value."""
    if isinstance(value, str):
        yield where, value
    elif isinstance(value, dict):
        for key, item in value.items():
            yield from iter_strings(item, f"{where}.{key}")
    elif isinstance(value, list):
        for index, item in enumerate(value):
            yield from iter_strings(item, f"{where}[{index}]")


def links_in(text: str) -> list[str]:
    """Target ids of the `[[…]]` links of a text (escapes `[[!…]]` excluded)."""
    return [
        match.strip()
        for match in LINK_PATTERN.findall(text)
        if not match.startswith(ESCAPE_PREFIX)
    ]


def escapes_in(text: str) -> list[str]:
    """Plain texts of the `[[!…]]` escapes of a text (never auto-linked in game)."""
    return [
        match.group(0)[3:-2]
        for match in LINK_PATTERN.finditer(text)
        if match.group(1).startswith(ESCAPE_PREFIX)
    ]


def load_todo_ids(codex_dir: Path) -> set[str]:
    """Ids of the entries planned in `_todo.md` (links to them are tolerated)."""
    todo = codex_dir / "_todo.md"
    if not todo.exists():
        return set()
    return set(TODO_ID_PATTERN.findall(todo.read_text(encoding="utf-8")))


def load_pending_ids(codex_dir: Path) -> set[str]:
    """Ids cited in every pending list `_*.md` (`_todo.md`, `_diet_links.md`...)."""
    ids: set[str] = set()
    for path in sorted(codex_dir.glob("_*.md")):
        ids |= set(TODO_ID_PATTERN.findall(path.read_text(encoding="utf-8")))
    return ids


def validate_codex(data_dir: Path) -> CodexReport:
    """Validates `data_dir/codex` and the codex links of every JSON file of `data_dir`."""
    report = CodexReport()
    codex_dir = data_dir / "codex"
    validator = schema_validator(data_dir, "codex.schema.json")
    report.todo_ids = load_todo_ids(codex_dir)

    for path in sorted(codex_dir.glob("*.json")):
        try:
            entry = json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError as error:
            report.errors.append(f"{path.name}: invalid JSON ({error})")
            continue
        for error in validator.iter_errors(entry):
            report.errors.append(f"{path.name}: {error.json_path}: {error.message}")
        if entry.get("id") != path.stem:
            report.errors.append(
                f"{path.name}: id {entry.get('id')!r} differs from file name"
            )
        report.entries[path.stem] = entry

    known = set(report.entries) | load_pending_ids(codex_dir)
    overlap = report.todo_ids & set(report.entries)
    if overlap:
        report.errors.append(
            f"_todo.md lists entries already written: {sorted(overlap)}"
        )

    _check_aliases(report)
    _check_exclude_contexts(report)
    for entry_id, entry in report.entries.items():
        for target in entry.get("see_also", []):
            if target not in known:
                report.errors.append(f"{entry_id}.see_also: unknown entry {target}")
        entity = entry.get("entity")
        if entity and not _entity_exists(data_dir, entity):
            report.errors.append(f"{entry_id}.entity: {entity} not found in data/")
    _check_decor(report, decor_keys(data_dir))

    for path in sorted(data_dir.rglob("*.json")):
        if "schemas" in path.parts or path.name == BUNDLE_NAME:
            continue
        try:
            content = json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            continue  # other validators report malformed files
        relative = path.relative_to(data_dir)
        for where, text in iter_strings(content, str(relative)):
            for target in links_in(text):
                report.link_count += 1
                if target not in known:
                    report.errors.append(f"{where}: unresolved link [[{target}]]")
            for escaped in escapes_in(text):
                if not escaped.strip():
                    report.errors.append(f"{where}: empty escape [[!]]")
    return report


def _check_aliases(report: CodexReport) -> None:
    """Aliases and titles must designate a single entry (case-insensitive)."""
    owners: dict[str, str] = {}
    for entry_id, entry in report.entries.items():
        forms = {str(entry.get("title", "")).casefold()}
        forms |= {str(alias).casefold() for alias in entry.get("aliases", [])}
        for form in forms:
            if not form:
                continue
            owner = owners.setdefault(form, entry_id)
            if owner != entry_id:
                report.errors.append(f"alias {form!r} shared by {owner} and {entry_id}")


def entry_forms(entry: dict) -> set[str]:
    """Title and aliases of an entry, case-folded (the forms the auto-link recognises)."""
    forms = {str(entry.get("title", "")).strip().casefold()}
    forms |= {str(alias).strip().casefold() for alias in entry.get("aliases", [])}
    return {form for form in forms if len(form) >= 2}


def _check_exclude_contexts(report: CodexReport) -> None:
    """Each excluded expression must contain the title or an alias of its entry."""
    for entry_id, entry in report.entries.items():
        forms = entry_forms(entry)
        for context in entry.get("exclude_contexts", []):
            if not any(form in str(context).casefold() for form in forms):
                report.errors.append(
                    f"{entry_id}.exclude_contexts: {context!r} contains no alias of the entry"
                )


def _entity_exists(data_dir: Path, entity: str) -> bool:
    """True if `data/<directory>/<entity>.json` exists for the entity prefix."""
    directory = ENTITY_DIRECTORIES.get(entity.split("_", 1)[0])
    return directory is not None and (data_dir / directory / f"{entity}.json").exists()


def decor_keys(data_dir: Path) -> set[str]:
    """Keys `<kind>:<id>` of every natural-decor species the game can show under the cursor."""

    def ids(items: object) -> list[str]:
        if isinstance(items, dict):
            return [str(key) for key in items]
        return [
            str(item["id"])
            for item in items or []
            if isinstance(item, dict) and "id" in item
        ]

    def load_json(relative: str) -> dict:
        return json.loads((data_dir / relative).read_text(encoding="utf-8"))

    rocks = yaml.safe_load(
        (data_dir / "art/rock_outcrops.yaml").read_text(encoding="utf-8")
    )
    sources = {
        "tree": ids(load_json("art/tree_species.json")["species"]),
        "battle_tree": ids(load_json("art/battle_tree_leaves.json")["species"]),
        "fauna": ids(load_json("map/map_fauna.json")["species"]),
        "bird": ids(load_json("map/map_birds.json")["species"]),
        "rock": ids(rocks["outcrops"]),
    }
    return {
        f"{kind}:{species}"
        for kind, species_ids in sources.items()
        for species in species_ids
    }


def _check_decor(report: CodexReport, known_keys: set[str]) -> None:
    """Each `decor` key must exist and belong to one entry only."""
    owners: dict[str, str] = {}
    for entry_id, entry in report.entries.items():
        for key in entry.get("decor", []):
            if key not in known_keys:
                report.errors.append(f"{entry_id}.decor: unknown species {key}")
            owner = owners.setdefault(key, entry_id)
            if owner != entry_id:
                report.errors.append(f"decor {key} shared by {owner} and {entry_id}")
