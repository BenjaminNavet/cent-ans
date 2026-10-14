# CP — carte peinte

## CP0 (infrastructure) : FAIT
Masques de retrait `_clear`, `compose/paint.py` + commande `compose paint [--preview out.png]`, `masks/derived/`,
recompilation complète de l'éditeur, calque `/api/cover/`, schéma `compose_paint.schema.json`, ADR 0361,
tests `test_compose_paint.py` + `test_compose_c4a.py`. Détail : `docs/editeur-carte.md`.

## Prochaine étape (CP1)
4 agents peignent chacun une région dans `data/map/compose/paint/<region>.json` : `ouest`, `mediterranee`, `est`, `orient`
(chaque trait avec une `note` en français). Contrôle : `uv run --project tools cent-ans compose paint --preview x.png`.
Puis, hors agents : `compose paint` + `geo compose-layers --only landcover,colormap` (bake complète, pas dans les tests).

## Points ouverts
- `agri_regions` ne lit pas les masques de champs.
- Bake réelle non lancée en CP0 (mémoire) : à vérifier au premier lot régional.
