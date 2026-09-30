# Lot SL1 — Routes maritimes

Branche `feat/sea-lanes`. ADR : `docs/decisions/0117-routes-maritimes.md`.

Demande : « il manque les routes maritimes dans le jeu (visuellement et avec des mécaniques de jeu) ».
Constat : le graphe des colonies n'a que de courts passages entre provinces voisines (Douvres–Wissant…) ;
aucune route Angleterre–Gascogne, rien de dessiné sur la mer, commerce maritime en traits droits à
travers les terres, maîtrise des mers (NV1) sans effet sur le commerce.

## État
| Étape | État |
|---|---|
| 1. Catalogue `data/naval/sea_lanes.json` (27 routes) + schéma + `cent-ans geo sea-lanes` → `data/map/sea_lanes_px.json` + `tools/tests/test_sea_lanes.py` | fait |
| 2. Cœur : routes dans le graphe, mer de la traversée, gros temps, commerce (maîtrise, blocus, saison), interception, vue `sea_lanes()` | à faire |
| 3. Pont `get_sea_lanes` + couche Godot `SeaLaneLayer`, infobulle, légende, commerce le long des routes | à faire |
| 4. ADR, manuel, vérifications | à faire |

## Prochaine étape
Étape 2 (data-model `SeaLanes`, `movement_graph`).
