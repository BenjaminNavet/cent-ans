# RS-G lot 1b — Avignon vers 1340 à l'échelle 1:1 (VH8)

Branche `feat/rs-g-cities` (worktree partagé avec les autres villes VH8). Fichiers du lot :
`data/landmarks_v2/avignon.json`, `tools/tests/test_landmarks_v2_avignon.py`, ce suivi.
Format : `docs/landmarks-v2.md` ; exemple Rouen. Cache OSM hors dépôt :
`tools/geo/raw/osm/avignon_*.json`.

## État
- [x] Squelette valide (origine sur l'ancre de la maquette v1, fleuve fin « Rhône »)
- [ ] Sources, enceintes datées et portes, pont Saint-Bénézet, monuments, quartiers, places
- [ ] Recette OSM et `cent-ans geo landmarks --city avignon`
- [ ] Test `test_landmarks_v2_avignon.py`

## Fleuve fin
La carte fine ne connaît qu'un « le Rhône » (normalisé « Rhône ») dans le rayon : une ligne de 300 m
de large le long du nord de la ville (bras d'Avignon). Pas de bras de Villeneuve fin dans le rayon.

## Prochaine étape
Recherche des sources, extraction OSM des repères, écriture du fichier.
