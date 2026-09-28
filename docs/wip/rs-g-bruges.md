# VH8 — Bruges vers 1340 à l'échelle 1:1 (format v2, ADR 0078)

Branche `feat/rs-g-cities` (worktree partagé avec trois autres villes). Référence :
`docs/landmarks-v2.md`, exemple Rouen. Fichiers du lot : `data/landmarks_v2/bruges.json`,
`tools/tests/test_landmarks_v2_bruges.py`, ce suivi.

## État
- [x] Squelette validant le schéma (origine sur le beffroi, EPSG:3035 [3848306, 3143840])
- [ ] Enceinte de 1297 (vesten) et portes, première enceinte (reien)
- [ ] Monuments, quartiers, espaces libres, eaux (reien à la main d'après OSM), rues OSM
- [ ] Sources, test des faits datés

## Notes
- Carte fine : aucun cours d'eau nommé utile (seul « Zuidervaartje » le long des vesten est) :
  `fine_rivers: []`, reien tracées à la main (`origin: "hand"`, `draw: true`) d'après OSM.
- Cache OSM hors dépôt : `tools/geo/raw/osm/bruges_features.json`, `bruges_features2.json`.

## Prochaine étape
Géométrie de l'enceinte et des reien depuis OSM, puis monuments.
