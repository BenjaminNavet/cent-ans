"""Règle d'import des textures 3D : VRAM compressé + mipmaps (lot QW-D, audit art 2026-10-09).

Godot extrait les textures des glb à côté des modèles (`<glb>_Image_0.jpg`) et écrit leur
`.import` avec `compress/mode=0` (sans perte) ; `detect_3d/compress_to=1` ne bascule jamais en
headless. Résultat : ~5,3 Mo de VRAM par 1024² au lieu de 0,7 Mo. Ces `.import` sont générés
(hors git pour `models/dn`) : la règle vit donc ici, appliquée après chaque import
(`tools/launch.sh`, `cent-ans art models-textures-fix`) et vérifiée par
`tools/tests/test_texture_import_rules.py`.

Périmètre : toute texture sous `game/assets/models/` et `game/assets/textures/water/` ; et les
tableaux de matières (`2d_array_texture`) de `game/assets/textures/` : VRAM + mipmaps, et pour les
tableaux de normales `compress/high_quality=true` (BC7 au lieu de BC1, qui faisait des blocs en
lumière rasante ; audit 08 § 3.2). Les nouveaux paquets (TX…) y sont soumis d'office.
"""

from __future__ import annotations

import re
from pathlib import Path

from cent_ans_tools.paths import GAME_DIR

#: Dossiers (relatifs à game/) dont les textures sont destinées au rendu 3D.
RULE_DIRS = ("assets/models", "assets/textures/water")
#: Dossier dont seuls les tableaux de matières sont soumis à la règle.
ARRAY_DIR = "assets/textures"

_NORMAL_NAME = re.compile(r"normal", re.IGNORECASE)
_PARAM = re.compile(r"^(?P<key>[a-z0-9_/]+)=(?P<value>.*)$")


def import_files(game_dir: Path = GAME_DIR) -> list[Path]:
    """Les `.import` de textures (hors scènes glb) du périmètre."""
    found: list[Path] = []
    for rel in RULE_DIRS:
        root = game_dir / rel
        if not root.is_dir():
            continue
        for path in sorted(root.rglob("*.import")):
            if path.name.endswith((".glb.import", ".gltf.import")):
                continue
            if 'importer="texture"' in path.read_text(
                encoding="utf-8", errors="replace"
            ):
                found.append(path)
    array_root = game_dir / ARRAY_DIR
    if array_root.is_dir():
        found += [
            path
            for path in sorted(array_root.rglob("*.import"))
            if _is_array(path) and path not in found
        ]
    return found


def _is_array(import_path: Path) -> bool:
    return 'importer="2d_array_texture"' in import_path.read_text(
        encoding="utf-8", errors="replace"
    )


def expected_params(import_path: Path) -> dict[str, str]:
    """Paramètres imposés (normales : `normal_map=1` → BC5 ; tableaux : BC7 haute qualité)."""
    params = {"compress/mode": "2", "mipmaps/generate": "true"}
    if not _NORMAL_NAME.search(import_path.name):
        return params
    if _is_array(import_path):
        params["compress/high_quality"] = "true"
    else:
        params["compress/normal_map"] = "1"
    return params


def violations(import_path: Path) -> list[str]:
    """Écarts (`clé=valeur actuelle`) entre le `.import` et la règle."""
    current: dict[str, str] = {}
    in_params = False
    for line in import_path.read_text(encoding="utf-8").splitlines():
        if line.startswith("["):
            in_params = line == "[params]"
        elif in_params and (match := _PARAM.match(line)):
            current[match["key"]] = match["value"]
    return [
        f"{key}={current.get(key, '<absent>')} (attendu {wanted})"
        for key, wanted in expected_params(import_path).items()
        if current.get(key) != wanted
    ]


def fix_import(import_path: Path) -> bool:
    """Réécrit les paramètres hors règle ; renvoie True si le fichier a changé."""
    if not violations(import_path):
        return False
    wanted = expected_params(import_path)
    lines = import_path.read_text(encoding="utf-8").splitlines()
    in_params = False
    for index, line in enumerate(lines):
        if line.startswith("["):
            in_params = line == "[params]"
        elif in_params and (match := _PARAM.match(line)) and match["key"] in wanted:
            lines[index] = f"{match['key']}={wanted[match['key']]}"
    import_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return True


def fix_all(game_dir: Path = GAME_DIR) -> list[Path]:
    """Applique la règle à tout le périmètre ; liste les `.import` modifiés."""
    return [path for path in import_files(game_dir) if fix_import(path)]
