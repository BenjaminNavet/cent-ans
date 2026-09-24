"""Navigation grid for free army movement (lot M1, spec 2026-09-24-mouvement-libre § 2)."""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

from cent_ans_tools.geo import settlements

MAP_DIR = settlements.MAP_DIR
NAVGRID_FILE = "navgrid.png"
CROSSINGS_FILE = "crossings.json"
NAVGRID_SIZE = 2048
IMPASSABLE = 255


@dataclass
class NavgridResult:
    """Output of :func:`build`."""

    path: Path
    preview: Path
    passable_fraction: float = 0.0
    crossings_used: int = 0
    warnings: list[str] = field(default_factory=list)


def build(map_dir: Path = MAP_DIR) -> NavgridResult:
    """Write ``navgrid.png``, its preview and ``map.json.navgrid``."""
    raise NotImplementedError
