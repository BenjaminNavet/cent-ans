# DC — Carte plus dense et plus lente

Demande du joueur (2026-09-26) : « la carte est trop petite / les armées vont trop vite », « plus de
villes pour une région donnée » ; accord sur B + C, « tu fais le plan ET les modifications ».
ADR 0082. Orchestrateur : session DC. Coût cloud : 0 $ (recherche et calcul locaux).

## Conventions
- Branche d'intégration `feat/densite` (worktree `../gp-densite`). Un worktree par lot :
  `../gp-dc1`, `../gp-dc2a`… créés par l'orchestrateur depuis `feat/densite`.
- Lots DC2 : **données seulement**, pas de build Rust ni Godot. Contrôle :
  `uv run --project tools python -m cent_ans_tools.geo.settlement_check <prov…>` et
  `uv run --project tools pytest tools/tests/test_settlements_schema.py`.
- Lot DC1 : `CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target` (disque ~29 Go libres).
- Commits `-- chemins` explicites, `wip:` ≤ 15 min ; fusion par l'orchestrateur.
- Coordination : la revue de code (`../gp-review`) et PB3d touchent `sim-campaign` (orders,
  movement, fin de tour) : fusionner `main` avant de rendre.

## Lots
| Lot | Contenu | Vague | État |
|---|---|---|---|
| DC0 | Squelette : ADR 0082, ce plan, plafond 6 → 16 (schéma, chargeur Rust, tests), outil `geo/settlement_check.py` | 0 | fait |
| DC1 | Mouvement : `points_per_step` 140 → 70 (rules.json + défaut Rust), vérifier traversées, horizon IA, agents, repli ; tests ; textes du codex et description de rules.json ; sondes `march_range_probe`, `turn_perf` | 1 | lancé |
| DC2a | Colonies France nord, ouest, centre (19 prov., cible 12) | 1 | lancé |
| DC2b | Colonies Aquitaine, Languedoc, France est, Provence-Alpes (22 prov., cible 11) | 1 | lancé |
| DC2c | Colonies îles Britanniques (Angleterre cible 10 ; Galles, Irlande, Écosse 7) | 1 | lancé |
| DC2d | Colonies Pays-Bas (11), Empire rhénan (9), reste de l'Empire et Scandinavie (7) | 1 | lancé |
| DC2e | Colonies Ibérie et Italie (cible 7-8) | 1 | lancé |
| DC1 | Mouvement : `points_per_step` 140 → 70 (rules.json + défaut Rust), vérifier traversées, horizon IA, agents, repli ; tests ; textes du codex et description de rules.json ; sondes `march_range_probe`, `turn_perf` | 1 | à lancer |
| DC2a | Colonies France nord, ouest, centre (19 prov., cible 12) | 1 | à lancer |
| DC2b | Colonies Aquitaine, Languedoc, France est, Provence-Alpes (22 prov., cible 11) | 1 | fait |
| DC2c | Colonies îles Britanniques (Angleterre cible 10 ; Galles, Irlande, Écosse 7) | 1 | à lancer |
| DC2d | Colonies Pays-Bas (11), Empire rhénan (9), reste de l'Empire et Scandinavie (7) | 1 | à lancer |
| DC2e | Colonies Ibérie et Italie (cible 7-8) | 1 | fait |
| DC3 | Régénération (`geo settlements`, `hamlets`, `anchors-fine`, `towns`), rangs de marqueurs, équilibrage économie/garnisons/entretien (Angleterre doit lever), IA et `turn_perf`, sim de 20 ans | 2 | après DC1 + DC2 |
| DC4 | Affichage : niveaux de détail des marqueurs, désencombrement des étiquettes (O(n²)), maquettes proches, captures | 2 | après DC2 |
| DC5 | Recette (orchestrateur) : build, smoke, cargo test, pytest, ff dans `main` | 3 | |

## Journal
- 26/09 : plan, squelette DC0 (6f74c214) ; vague 1 lancée (DC1 opus, DC2a-e sonnet, worktrees ../gp-dc1, ../gp-dc2a-e).
- 26/09 : DC2c : 105 colonies ajoutées (îles Britanniques ; village 43, abbey 27, town 18, castle 17). Fusionné.
- 26/09 : DC2d : 139 colonies ajoutées (village 52, town 55, abbey 24, castle 8) dans les 33
  provinces Pays-Bas/Empire/Scandinavie ; `settlement_check` et `pytest test_settlements_schema.py`
  passent (134 passed, aucun id en double, aucune paire <3 km introduite).
- 26/09 : plan, squelette DC0.
- 26/09 : DC2a : 128 colonies ajoutées (31 town, 15 castle, 15 abbey, 67 village) sur 19 provinces de
  France nord/ouest/centre ; `settlement_check` et `test_settlements_schema.py` passent.
- 26/09 : DC2e : 118 colonies ajoutées (52 town, 44 village, 12 abbey, 10 castle) sur 34 provinces
  d'Ibérie et d'Italie (Portugal, Aragon, Ibérie nord/centre/sud, Italie nord/centre/sud) ; Mallorca et
  Roussillon déjà à leur cible n'ont reçu aucun ajout côté Roussillon (7/7), 3 côté Mallorca (6/6).
  `settlement_check` et `pytest tools/tests/test_settlements_schema.py` verts sur les 35 provinces.
## Prochaine étape
Attendre la vague 1 ; fusionner DC2a-e (fichiers disjoints) puis DC1 dans feat/densite ; lancer DC3 + DC4.
- 26/09 : DC2b : 132 colonies ajoutées sur 22 provinces (Aquitaine 49, Languedoc 33, France est 32,
  Provence-Alpes 18) — village 48, town 46, abbey 22, castle 16.
  `settlement_check` et `pytest tools/tests/test_settlements_schema.py`
  passent sur les 22 provinces. Comtat Venaissin et Dauphiné restent sous la cible (7 et 9 au lieu de
  7 et 10) faute de lieux attestés supplémentaires assez notables (provinces alpines pauvres) ; conforme
  à la consigne « adapte, ne force pas ». Anomalies préexistantes signalées, non corrigées (hors mandat) :
  `set_vaucouleurs` (prov_bar), `set_cognac`/`set_aubeterre` (prov_angoumois), `set_peyrepertuse`
  (prov_carcassonne), `set_valmagne` + paire `set_montpellier`/`set_castelnau_le_lez` à 2 km
  (prov_montpellier), `set_senanque` (prov_comtat_venaissin), `set_vienne` (prov_dauphine),
  `set_tarascon`/`set_les_baux` (prov_provence), `set_figeac` (prov_quercy), `set_la_rochelle`
  (prov_saintonge) — tous signalés « hors province (sera recalée) » par l'outil géographique, points
  historiquement corrects sur des frontières de province qui suivent une géométrie différente ;
  `set_bergerac` (prov_perigord) a un commentaire de liberté de jeu assumé (Anglais dès 1337) à
  vérifier par un lot d'équilibrage.
Lancer/poursuivre la vague 1 (DC1, DC2a, DC2c-e) ; DC2b terminé, prêt pour fusion dans `feat/densite`.

## DC4 — Affichage (worktree ../gp-dc4, branche feat/densite-dc4)
État : sonde `game/tests/dc4_density_probe.gd` écrite (fenêtrée : temps d'image, temps de
`declutter()`, marqueurs et étiquettes à l'écran, chevauchements, maquettes voisines, hameaux sur
colonies, travelling au palier vallée ZG6). Mesures « avant » en cours.
Build : `CARGO_TARGET_DIR` partagé ; attention, `deps/libcent_ans.dylib` n'a pas de hachage et est
écrasé par la dernière construction de n'importe quel worktree : `touch
core/crates/godot-bridge/src/lib.rs` avant `cargo build`, puis copier aussitôt.
Prochaine étape : mesures avant (570 et 1 192), rangs, grille spatiale du désencombrement.
