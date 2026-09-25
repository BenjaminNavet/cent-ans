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
- **E1-E2 PRÊTES (pour ZG2)** : 313 tuiles E1 (91 Mo) + 1 069 tuiles E2 (≈ 310 Mo), 405 Mo au
  total, 51 s sur 14 cœurs. Manifeste `tiles_rle` des étages 1-2 à jour. Cache partagé :
  `/Users/jean_hubert/dev/game_project/data/map/pyramid/E1`, `E2`.
- Lecture GLO-90 : tuiles 1° **mosaïquées avant reprojection** (`glo30.mosaic_to_grid`) ;
  la méthode d'E0 (`copernicus.resample_to_grid`, tuile par tuile) laisse des coutures le long
  de chaque méridien et parallèle entier (jusqu'à ~300 m d'écart dans les Alpes, visible en
  ombrage). E0 n'est pas touché (hors lot) : écart E0 / moyenne 2 × 2 d'E1 sur terre : moyenne
  +0,03 m, médiane |d| 0,48 m, p95 7,6 m, p99 17 m, max 295 m (coutures d'E0) ; E1 / E2 :
  exact à la quantification (p99 0,06 m).

## Prochaine étape
- Signaler à l'orchestrateur : les coutures d'E0 (régénérer `geo relief-shade` avec la
  mosaïque serait un autre lot).
- `geo/surface.py` : correction de surface GLO-30 (canopée WorldCover, bâti, retenues de
  `data/map/modern_reservoirs.json`), palier 2 (E3-E4).
- Tests, aperçus `docs/img/zg1/`, `docs/geo.md`, `CREDITS.md`.

## Mesures
- Bruts : GLO-30 5,18 Go + WorldCover 1,54 Go (+ GLO-90 0,96 Go déjà là).
