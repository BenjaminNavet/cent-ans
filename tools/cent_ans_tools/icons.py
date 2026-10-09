"""Game icons (F2): download game-icons.net SVGs, tint them sepia ink, write the table.

Sources: https://github.com/game-icons/icons (CC BY 3.0). Every icon of
``icons_catalog.ICONS`` is fetched once from ``raw.githubusercontent.com`` into a
local cache (``~/.cache/cent-ans/game-icons`` by default, shared by worktrees),
then normalised:

- the black 512x512 background square is removed;
- every filled shape is painted in ``INK`` (the parchment theme's brown ink);
- the root gets ``width``/``height`` = ``ICON_SIZE`` so that Godot imports a
  texture of that size (the ``viewBox`` keeps the vector at any scale).

Output: ``game/assets/icons/<author>-<name>.svg`` (one file per distinct source)
and ``game/assets/icons/icons.json`` (versioned) mapping each id to its file,
category, source and author, plus the category fallbacks. Idempotent: the cache
avoids network calls and unchanged files are not rewritten. No cloud spending.
"""

from __future__ import annotations

import json
import re
import xml.etree.ElementTree as ET
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

import httpx

from cent_ans_tools import icons_catalog
from cent_ans_tools.paths import REPO_DIR

ICONS_DIR = REPO_DIR / "game" / "assets" / "icons"
DATA_DIR = REPO_DIR / "data"
DEFAULT_CACHE_DIR = Path.home() / ".cache" / "cent-ans" / "game-icons"
RAW_URL = "https://raw.githubusercontent.com/game-icons/icons/master/{source}.svg"
LICENSE = "CC BY 3.0"
LICENSE_URL = "https://creativecommons.org/licenses/by/3.0/"
SITE_URL = "https://game-icons.net"

# Brown ink of the parchment theme (Label font 0.22/0.14/0.07, borders 0.42/0.29/0.16).
INK = "#4a3219"
ICON_SIZE = 64

SVG_NS = "http://www.w3.org/2000/svg"
BACKGROUND_PATHS = {"M0 0h512v512H0z", "M0 0h512v512H0V0z"}

Fetcher = Callable[[str], str]


@dataclass(frozen=True)
class IconEntry:
    """One row of ``icons.json``."""

    id: str
    file: str
    category: str
    source: str
    author: str


def author_of(source: str) -> str:
    """Credited author name of a ``<author>/<name>`` source."""
    slug = source.split("/", 1)[0]
    return icons_catalog.AUTHORS.get(slug, slug.replace("-", " ").title())


def file_name_of(source: str) -> str:
    """Output file name of a source: ``lorc/crossed-swords`` -> ``lorc-crossed-swords.svg``."""
    return source.replace("/", "-") + ".svg"


def http_fetcher(timeout: float = 20.0) -> Fetcher:
    """Fetcher downloading raw SVGs from GitHub (free, no authentication)."""
    client = httpx.Client(timeout=timeout, follow_redirects=True)

    def fetch(source: str) -> str:
        response = client.get(RAW_URL.format(source=source))
        response.raise_for_status()
        return response.text

    return fetch


def cached_svg(source: str, cache_dir: Path, fetch: Fetcher | None) -> str:
    """Raw SVG of ``source`` from the cache, downloading it on a miss.

    Raises:
        FileNotFoundError: the icon is not cached and no fetcher is given (offline).
    """
    path = cache_dir / f"{source}.svg"
    if path.exists():
        return path.read_text(encoding="utf-8")
    if fetch is None:
        raise FileNotFoundError(f"icon {source} not cached in {cache_dir}")
    text = fetch(source)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    return text


def normalize_svg(svg_text: str, color: str = INK, size: int = ICON_SIZE) -> str:
    """Monochrome ``color`` SVG of ``size`` px without the square background.

    Raises:
        ValueError: the document declares a DTD or entities (never the case for
            game-icons; refused so that the stdlib parser cannot be abused).
    """
    if "<!DOCTYPE" in svg_text or "<!ENTITY" in svg_text:
        raise ValueError("SVG with DTD/entities refused")
    ET.register_namespace("", SVG_NS)
    root = ET.fromstring(svg_text)
    for parent in list(root.iter()):
        for child in list(parent):
            tag = child.tag.rsplit("}", 1)[-1]
            if tag == "path" and child.get("d", "").strip() in BACKGROUND_PATHS:
                parent.remove(child)
    for element in root.iter():
        tag = element.tag.rsplit("}", 1)[-1]
        if "style" in element.attrib:
            element.set(
                "style",
                re.sub(r"fill\s*:\s*[^;]+", f"fill:{color}", element.get("style", "")),
            )
        fill = element.get("fill")
        shape = tag in {"path", "circle", "rect", "ellipse", "polygon"}
        if (fill is not None and fill != "none") or (fill is None and shape):
            element.set("fill", color)
    root.set("width", str(size))
    root.set("height", str(size))
    if "viewBox" not in root.attrib:
        root.set("viewBox", "0 0 512 512")
    return ET.tostring(root, encoding="unicode") + "\n"


def entries() -> list[IconEntry]:
    """Every icon of the catalogue, sorted by id."""
    return [
        IconEntry(
            id=icon_id,
            file=file_name_of(source),
            category=category,
            source=source,
            author=author_of(source),
        )
        for icon_id, (source, category) in sorted(icons_catalog.ICONS.items())
    ]


def table(rows: list[IconEntry]) -> dict:
    """Content of ``icons.json``."""
    return {
        "license": LICENSE,
        "license_url": LICENSE_URL,
        "site": SITE_URL,
        "color": INK,
        "size": ICON_SIZE,
        "fallbacks": dict(sorted(icons_catalog.FALLBACKS.items())),
        "icons": {
            row.id: {
                "file": row.file,
                "category": row.category,
                "source": row.source,
                "author": row.author,
            }
            for row in rows
        },
    }


def _write_if_changed(path: Path, text: str) -> bool:
    if path.exists() and path.read_text(encoding="utf-8") == text:
        return False
    path.write_text(text, encoding="utf-8")
    return True


# Minimal Godot import settings: mipmaps so that 64 px icons stay smooth at 16-28 px.
IMPORT_TEMPLATE = """[remap]

importer="texture"
type="CompressedTexture2D"

[params]

mipmaps/generate=true
svg/scale=1.0
"""


def ensure_import_settings(svg_path: Path) -> bool:
    """Creates ``<svg>.import`` (mipmaps on) or turns mipmaps on in an existing one."""
    import_path = svg_path.with_name(svg_path.name + ".import")
    if not import_path.exists():
        import_path.write_text(IMPORT_TEMPLATE, encoding="utf-8")
        return True
    text = import_path.read_text(encoding="utf-8")
    if "mipmaps/generate=false" in text:
        import_path.write_text(
            text.replace("mipmaps/generate=false", "mipmaps/generate=true"),
            encoding="utf-8",
        )
        return True
    return False


def build(
    out_dir: Path = ICONS_DIR,
    cache_dir: Path = DEFAULT_CACHE_DIR,
    fetch: Fetcher | None = None,
    offline: bool = False,
) -> tuple[list[IconEntry], int]:
    """Writes the normalised SVGs and ``icons.json``; returns (entries, files written).

    Args:
        out_dir: target directory (``game/assets/icons``).
        cache_dir: raw SVG cache.
        fetch: downloader (defaults to :func:`http_fetcher`); ignored when offline.
        offline: never touch the network (cache misses raise).
    """
    rows = entries()
    out_dir.mkdir(parents=True, exist_ok=True)
    if offline:
        fetch = None
    elif fetch is None:
        fetch = http_fetcher()
    written = 0
    for source in sorted({row.source for row in rows}):
        svg = normalize_svg(cached_svg(source, cache_dir, fetch))
        written += _write_if_changed(out_dir / file_name_of(source), svg)
        written += ensure_import_settings(out_dir / file_name_of(source))
    text = json.dumps(table(rows), ensure_ascii=False, indent=2, sort_keys=False) + "\n"
    written += _write_if_changed(out_dir / "icons.json", text)
    return rows, written


def data_ids(data_dir: Path, directory: str) -> list[str]:
    """Ids of the JSON files of ``data/<directory>``."""
    ids = []
    for path in sorted((data_dir / directory).glob("*.json")):
        ids.append(json.loads(path.read_text(encoding="utf-8"))["id"])
    return ids


def missing_icons(data_dir: Path = DATA_DIR) -> list[str]:
    """Data ids (or category keys) without an icon; empty when coverage is complete."""
    missing = []
    for directory in icons_catalog.EXPLICIT_DATA_DIRS:
        missing += [
            i for i in data_ids(data_dir, directory) if i not in icons_catalog.ICONS
        ]
    for directory, (field, prefix) in icons_catalog.CATEGORY_DATA_DIRS.items():
        for path in sorted((data_dir / directory).glob("*.json")):
            value = json.loads(path.read_text(encoding="utf-8")).get(field, "")
            if f"{prefix}{value}" not in icons_catalog.ICONS:
                missing.append(f"{prefix}{value}")
    for fallback in icons_catalog.FALLBACKS.values():
        if fallback not in icons_catalog.ICONS:
            missing.append(fallback)
    return sorted(set(missing))


def credits_rows(rows: list[IconEntry] | None = None) -> dict[str, list[str]]:
    """Author -> sorted distinct icon names (for ``CREDITS.md``)."""
    by_author: dict[str, set[str]] = {}
    for row in rows or entries():
        by_author.setdefault(row.author, set()).add(row.source.split("/", 1)[1])
    return {author: sorted(names) for author, names in sorted(by_author.items())}
