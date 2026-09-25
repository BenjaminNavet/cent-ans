# ZG1 — pyramide de relief, paliers 1-2 (E1-E4)

ADR 0036. Commande : `uv run --project tools cent-ans geo pyramid [--levels 1,2,3,4] [--force] [--workers N] [--limit N]`.
Branche : `worktree-agent-a0df650280bb11446`.

## État
- `geo/pyramid.py` : géométrie des étages, RLE, mise à jour partielle du manifeste
  (`update_manifest_levels`, une ligne par étage + ligne `cache`), palier 1 (E1-E2 par tuile E1,
  E1 = moyenne 2 × 2 d'E2), reprise (tuiles présentes sautées), multi-processus.
- `geo/glo30.py` : téléchargement GLO-30 (172 tuiles, 5,18 Go) et WorldCover (23 tuiles,
  1,54 Go) du cœur, faits (≈ 6,5 min).
- Base du rehaussement : `tools/geo/raw/pyramid_work/base.npy` (flou σ 5 km de la mosaïque
  Copernicus + ETOPO 8192², celle d'E0). Vérifié : E0 se reconstruit à la quantification près
  (écart max 0,038 m) à partir de cette base.
- Tuiles candidates (terre d'E0 dans l'emprise) : E1 313, E2 1 069, E3 2 297, E4 8 591.
- **E1-E2 : cuisson complète en cours** (voir journal).

## Prochaine étape
- `geo/surface.py` : correction de surface GLO-30 (canopée WorldCover, bâti, retenues de
  `data/map/modern_reservoirs.json`), palier 2 (E3-E4).
- Tests, aperçus `docs/img/zg1/`, `docs/geo.md`, `CREDITS.md`.

## Mesures
- Bruts : GLO-30 5,18 Go + WorldCover 1,54 Go (+ GLO-90 0,96 Go déjà là).
