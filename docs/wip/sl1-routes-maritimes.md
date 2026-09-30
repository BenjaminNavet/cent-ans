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
| 2. Cœur : routes dans le graphe, mer de la traversée, gros temps, commerce (maîtrise, blocus, saison), interception, vue `sea_lanes()` | fait, `sim-campaign/tests/sl1_sea_lanes.rs` |
| 3. Pont `get_sea_lanes`/`is_sea_link` + couche Godot `SeaLaneLayer`, infobulle, légende, commerce le long des routes, embarquement au clic droit, smoke | écrit, non exécuté (pas de Godot dans l'environnement) |
| 4. ADR, manuel, vérifications | ADR et manuel faits ; vérifications en cours |

## Prochaine étape
Suite complète `cargo test`, clippy, build du pont ; puis `godot --headless --path game --import` et `smoke.gd` sur une machine avec Godot.
