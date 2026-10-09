"""Chemins communs des outils : racine du dépôt et dossiers de premier niveau."""

from pathlib import Path

TOOLS_DIR = Path(__file__).resolve().parents[1]
REPO_DIR = TOOLS_DIR.parent
DATA_DIR = REPO_DIR / "data"
DOCS_DIR = REPO_DIR / "docs"
GAME_DIR = REPO_DIR / "game"
