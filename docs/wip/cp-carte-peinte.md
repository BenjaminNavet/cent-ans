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

## Contrôle CP1
`compose paint --check` (4 fichiers, 180 traits) : 0 sommet hors domaine, 0 trait ignoré, 0 chevauchement add/clear entre fichiers
régionaux (ouest/mediterranee dans le Languedoc compris). Intra-fichier (volontaire, retrait dans un ajout) : forest
mediterranee 31 cases, fields orient 4, rock ouest 4. `compose paint --preview` OK (le NaN initial venait de sommets désormais
corrigés ; la rastérisation ignore désormais tout sommet hors domaine avec avertissement). Masques dérivés commités :
`masks/derived/*.png` (forest 20 traits, forest_clear 36, fields 61, fields_clear 7, rock 12, rock_clear 5, marsh 39).
Reste : bake `geo compose-layers --only landcover,colormap`.
