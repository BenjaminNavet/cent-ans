"""NB: Nano Banana 2 redraw of the illuminated UI kit, guided by shape and style.

Pipeline (spec ``docs/superpowers/specs/2026-09-30-nb-nano-banana-interface-design.md``):
each piece of ``game/assets/ui/illumination/kit.json`` is sent to the image model with
two references, the style anchor (``data/art/style/anchor.png``) and the current
procedural texture upscaled on a green background (shape guide). The raw answer is
keyed out, fitted back to the piece geometry, its stretched bands made seamless, and
written to ``game/assets/ui/illumination/nb/<id>.png`` where ``ui_illumination.build``
picks it up instead of the procedural drawing.
"""

from __future__ import annotations

from dataclasses import dataclass
from decimal import Decimal
from pathlib import Path
from typing import Any

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
ORNAMENTS_PATH = ROOT / "data" / "art" / "ui_ornaments.yaml"
ANCHOR_PATH = ROOT / "data" / "art" / "style" / "anchor.png"
KIT_DIR = ROOT / "game" / "assets" / "ui" / "illumination"
NB_DIR = KIT_DIR / "nb"
RAW_DIR = ROOT / "tools" / "nb_raw" / "ui"

# Ledger section every NB spend lands in, and its own envelope (spec § 2).
NB_SECTION = "Nano Banana NB"
NB_CAP = Decimal("10.00")
# Pure green used as the keyable background of generated pieces.
KEY_GREEN = (0, 255, 0)


@dataclass(frozen=True)
class OrnamentRequest:
    """One paid generation: a kit piece (or decor) variant with its references."""

    piece_id: str
    variant: int
    prompt: str
    references: tuple[bytes, ...]
    image_config: dict[str, str]
    seed: int


def load_ornaments(path: Path = ORNAMENTS_PATH) -> dict[str, Any]:
    """Return the parsed ``ui_ornaments.yaml`` document."""
    raise NotImplementedError


def build_requests(
    kit: dict[str, Any], anchor: bytes, config: dict[str, Any]
) -> list[OrnamentRequest]:
    """Build every request from the kit geometry, the anchor and the YAML config."""
    raise NotImplementedError


def generate(
    requests: list[OrnamentRequest], raw_dir: Path = RAW_DIR, dry_run: bool = False
) -> list[Path]:
    """Paid calls (budget-guarded); raw images already in ``raw_dir`` are reused."""
    raise NotImplementedError


def key_out(image: np.ndarray) -> np.ndarray:
    """Return RGBA with the green background removed and green fringes decontaminated."""
    raise NotImplementedError


def fit_to_piece(image: np.ndarray, piece: dict[str, Any]) -> np.ndarray:
    """Crop to the opaque bounds and resize to the piece ``size`` of ``kit.json``."""
    raise NotImplementedError


def seam_fix(image: np.ndarray, piece: dict[str, Any]) -> np.ndarray:
    """Make the stretched central bands (between 9-slice margins) seamless."""
    raise NotImplementedError


def contact_sheet(before: list[Path], after: list[Path], out_path: Path) -> Path:
    """Write a before/after review sheet of the kit pieces."""
    raise NotImplementedError
