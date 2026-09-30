# Lot SL1 — Routes maritimes

Branche `feat/sea-lanes`. ADR : `docs/decisions/0139-routes-maritimes.md`.

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
| 4. ADR, manuel, vérifications | fait : `cargo test` data-model / sim-campaign / ai verts, clippy -D warnings vert sur les crates touchées, pytest vert |

## Prochaine étape
Sur une machine avec Godot : `core/build.sh`, `godot --headless --path game --import`, puis
`smoke.gd` (vérification `smoke OK: sea lanes`) et une capture de contrôle de la couche (le GDScript
n'a pas pu être exécuté dans l'environnement du lot).

## Retouches de tests existants
- `ai/tests/c7a.rs::the_ai_wins_back_a_lost_place` : l'armée anglaise quitte Londres (à une traversée de
  Calais, la France défendait Calais d'abord, à juste titre).
- `ai/tests/cv3_ai_stances.rs::the_stance_ai_is_deterministic` : graine 2 (la graine 1 n'avait plus de
  posture au tour 24 ; le test vérifie le déterminisme).
- `sim-campaign/tests/nv2_naval_campaign.rs` : Calais→Londres se livre dans la Manche (mer de la route).
- IA : `SEA_THREAT_FACTOR` = 0,5 (menace d'une armée de l'autre côté de la mer).

## Pièges
- `cargo clippy -- -D warnings` échoue déjà sur `main` : `sim-battle/src/sim.rs:3096` (`nonminimal_bool`).
