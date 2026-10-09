"""Catalogue loading and schema validation (T1c)."""

from __future__ import annotations

import json
from pathlib import Path

import yaml
from jsonschema import Draft202012Validator

from cent_ans_tools.texture_factory import CATALOG_DIR

SCHEMA_PATH = (
    Path(__file__).resolve().parents[3]
    / "data"
    / "schemas"
    / "texture_catalog.schema.json"
)


class CatalogError(ValueError):
    """Catalogue absent, illisible ou non conforme au schéma."""


def load_catalog(family: str, catalog_dir: Path = CATALOG_DIR) -> dict:
    """Read ``<catalog_dir>/<family>.yaml`` and validate it against the schema."""
    path = Path(catalog_dir) / f"{family}.yaml"
    if not path.is_file():
        raise CatalogError(f"Catalogue introuvable : {path}")
    try:
        document = yaml.safe_load(path.read_text(encoding="utf-8"))
    except yaml.YAMLError as error:
        raise CatalogError(f"{path} : YAML invalide ({error})") from error
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    errors = sorted(
        Draft202012Validator(schema).iter_errors(document),
        key=lambda error: [str(part) for part in error.absolute_path],
    )
    if errors:
        details = "; ".join(
            f"{'/'.join(str(part) for part in error.absolute_path) or '<racine>'}"
            f" : {error.message}"
            for error in errors[:5]
        )
        raise CatalogError(f"{path} invalide : {details}")
    if document["family"] != family:
        raise CatalogError(
            f"{path} : family={document['family']!r}, attendu {family!r}"
        )
    ids = [entry["id"] for entry in document["entries"]]
    duplicates = sorted({entry_id for entry_id in ids if ids.count(entry_id) > 1})
    if duplicates:
        raise CatalogError(f"{path} : identifiants en double {duplicates}")
    return document


def select(document: dict, only: list[str] | None = None) -> list[dict]:
    """Entries to process: all, or those whose id is in ``only`` (unknown id = error)."""
    entries = document["entries"]
    if not only:
        return list(entries)
    known = {entry["id"] for entry in entries}
    unknown = [entry_id for entry_id in only if entry_id not in known]
    if unknown:
        raise CatalogError(f"Identifiants inconnus : {unknown}")
    return [entry for entry in entries if entry["id"] in only]


def build_prompt(document: dict, entry: dict) -> str:
    """Full prompt: style prefix + entry prompt + style suffix; ``{tile_m}`` substituted."""
    parts = [document["style_prefix"], entry["prompt"], document["style_suffix"]]
    text = " ".join(part.strip() for part in parts if part.strip())
    return text.replace("{tile_m}", f"{entry['tile_m']:g}")
