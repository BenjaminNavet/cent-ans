"""CB-M2 : mesure du trajet en pointillés sur les rendus de `cbm2_path_shot.gd --probe=<dossier>`.

Compare le rendu trajet visible au rendu trajet masqué au milieu des tirets (`dash`) et à 6 m de
côté (`aside`), avec le bruit de fond (deux rendus masqués). Un trajet correct : tirets nettement
teintés, côté inchangé.

Usage : python3 game/tests/cbm2_path_probe.py <dossier>
"""

import json
import sys
from pathlib import Path

from PIL import Image

THRESHOLD = 12.0  # écart moyen RVB (0-255) au-delà duquel un pixel est « teinté »


def _zone(first: Image.Image, second: Image.Image, points: list) -> dict:
    """Écart moyen et part de points teintés entre deux rendus."""
    diffs = []
    for x, y in points:
        a = first.getpixel((x, y))
        b = second.getpixel((x, y))
        diffs.append(sum(abs(p - q) for p, q in zip(a, b, strict=True)) / 3.0)
    mean = sum(diffs) / len(diffs) if diffs else 0.0
    tinted = sum(1 for d in diffs if d > THRESHOLD) / len(diffs) if diffs else 0.0
    return {
        "points": len(diffs),
        "mean_diff": round(mean, 1),
        "tinted": round(tinted, 2),
    }


def measure(folder: Path) -> dict:
    """Mesure du trajet : tirets, côté, bruit."""
    info = json.loads((folder / "probe-path.json").read_text())
    on = Image.open(folder / "probe-path-on.png").convert("RGB")
    off = Image.open(folder / "probe-path-off.png").convert("RGB")
    off2 = Image.open(folder / "probe-path-off2.png").convert("RGB")
    return {
        "length_m": round(info["length"], 1),
        "dash": _zone(on, off, info["dash"]),
        "aside": _zone(on, off, info["aside"]),
        "noise_dash": _zone(off, off2, info["dash"]),
    }


def main() -> None:
    """Affiche la mesure et échoue si les tirets ne se voient pas."""
    result = measure(Path(sys.argv[1]))
    print(json.dumps(result))
    visible = result["dash"]["tinted"] >= 0.8 and result["aside"]["tinted"] <= 0.1
    sys.exit(0 if visible else 1)


if __name__ == "__main__":
    main()
