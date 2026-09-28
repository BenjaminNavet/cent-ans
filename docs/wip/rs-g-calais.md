# RS-G / VH8 — Calais vers 1340 à l'échelle 1:1 (format v2)

Branche `feat/rs-g-cities` (worktree partagé avec d'autres villes). Fichiers : `data/landmarks_v2/calais.json`,
`tools/tests/test_landmarks_v2_calais.py`, ce suivi. Caches OSM (non versionnés) :
`tools/geo/raw/osm/calais_highways.json`, `calais_features.json`, `calais_features2.json`.

## État
- [x] squelette validant le schéma
- [x] recherche des sources (SRA Hauts-de-France 2025, Réseau Vauban 2015, Greaves 1918, Mérimée,
      Wikipédia FR/EN, vue de 1558 et plan de 1693 en contrôle humain)
- [x] enceinte (1228) + seconde enceinte, 4 portes, château, Notre-Dame en 3 phases, tour du Guet,
      hôtel de ville, Saint-Nicolas, Étape (1363), Rysbank (1347 / 1400), avant-portes, quartiers,
      havre restitué, 52 rues OSM + 3 chemins à la main
- [ ] test des faits datés (`test_landmarks_v2_calais.py`)
- [ ] relecture finale, commit `feat(vh8)`

## Origine
Origine à 150 m au nord de la tour du Guet (front de havre, axe de la rue du Havre) : ≈ 283 m de
l'ancre v1 (< 300 m). L'ancre v1 tombe dans le havre.

## Prochaine étape
Écrire le test des faits, relire les descriptions, commit final.
