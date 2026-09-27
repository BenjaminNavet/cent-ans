"""CB-M1 : mesure du contour de décale sur les rendus de `cbm_outline_shot.gd --probe=<dossier>`.

Pour chaque décale, compare le rendu décales visibles au rendu décales masquées aux points écran
du cœur (30 % central) et de la bande du bord. Un cadre correct : cœur inchangé (écart faible),
bord nettement teinté.

Usage : python3 game/tests/cbm_outline_probe.py <dossier>
"""

import json
import sys
from pathlib import Path

from PIL import Image

THRESHOLD = 12.0  # écart moyen RVB (0-255) au-delà duquel un pixel est « teinté »


def measure(folder: Path, name: str) -> dict:
    """Écart moyen et part de pixels teintés au cœur et au bord d'une décale."""
    info = json.loads((folder / f"probe-{name}.json").read_text())
    on = Image.open(folder / f"probe-{name}-on.png").convert("RGB")
    off = Image.open(folder / f"probe-{name}-off.png").convert("RGB")
    result = {"size_m": [round(value, 1) for value in info["size"]]}
    pairs = [("", on, off)]
    baseline = folder / f"probe-{name}-off2.png"
    if baseline.exists():
        pairs.append(("noise_", off, Image.open(baseline).convert("RGB")))
    for prefix, first, second in pairs:
        for zone in ("core", "edge"):
            diffs = []
            for x, y in info[zone]:
                a = first.getpixel((x, y))
                b = second.getpixel((x, y))
                diffs.append(sum(abs(p - q) for p, q in zip(a, b, strict=True)) / 3.0)
            mean = sum(diffs) / len(diffs) if diffs else 0.0
            tinted = (
                sum(1 for d in diffs if d > THRESHOLD) / len(diffs) if diffs else 0.0
            )
            result[prefix + zone] = {
                "points": len(diffs),
                "mean_diff": round(mean, 1),
                "tinted": round(tinted, 2),
            }
    return result


def main() -> None:
    """Affiche la mesure de chaque décale sondée."""
    folder = Path(sys.argv[1])
    for path in sorted(folder.glob("probe-*.json")):
        name = path.stem.removeprefix("probe-")
        print(name, json.dumps(measure(folder, name)))


if __name__ == "__main__":
    main()
