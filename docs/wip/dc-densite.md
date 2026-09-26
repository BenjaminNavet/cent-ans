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
| DC2a | Colonies France nord, ouest, centre (19 prov., cible 12) | 1 | fait, fusionné |
| DC2b | Colonies Aquitaine, Languedoc, France est, Provence-Alpes (22 prov., cible 11) | 1 | fait, fusionné |
| DC2c | Colonies îles Britanniques (Angleterre cible 10 ; Galles, Irlande, Écosse 7) | 1 | fait, fusionné |
| DC2d | Colonies Pays-Bas (11), Empire rhénan (9), reste de l'Empire et Scandinavie (7) | 1 | fait, fusionné |
| DC2e | Colonies Ibérie et Italie (cible 7-8) | 1 | fait, fusionné |
| DC1 | Mouvement : `points_per_step` 140 → 70 (rules.json + défaut Rust), vérifier traversées, horizon IA, agents, repli ; tests ; textes du codex et description de rules.json ; sondes `march_range_probe`, `turn_perf` | 1 | à lancer |
| DC2a | Colonies France nord, ouest, centre (19 prov., cible 12) | 1 | à lancer |
| DC2b | Colonies Aquitaine, Languedoc, France est, Provence-Alpes (22 prov., cible 11) | 1 | fait |
| DC2c | Colonies îles Britanniques (Angleterre cible 10 ; Galles, Irlande, Écosse 7) | 1 | à lancer |
| DC2d | Colonies Pays-Bas (11), Empire rhénan (9), reste de l'Empire et Scandinavie (7) | 1 | à lancer |
| DC2e | Colonies Ibérie et Italie (cible 7-8) | 1 | fait |
| DC3 | Régénération (`geo settlements`, `hamlets`, `anchors-fine`, `towns`), rangs de marqueurs, équilibrage économie/garnisons/entretien (Angleterre doit lever), IA et `turn_perf`, sim de 20 ans | 2 | après DC1 + DC2 |
| DC4 | Affichage : niveaux de détail des marqueurs, désencombrement des étiquettes (O(n²)), maquettes proches, captures | 2 | lancé (../gp-dc4) |
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
- 26/09 : DC2 fusionné : 1 192 colonies (city 132, town 407, village 291, abbey 189, castle 173). Régénéré : graphe (2 987 arêtes), positions, fine_anchors, hameaux, towns_1340 (1 185). À surveiller DC3 : +66 châteaux (entretien) ; côte est du Sussex dans le polygone du Kent. DC4 lancé.

## Prochaine étape
Attendre DC1, le fusionner, puis lancer DC3 (équilibrage) ; fusionner DC4.
Lancer la vague 1 (DC1 + DC2a-e, 6 agents).
## DC1 — mouvement ralenti (worktree `../gp-dc1`, branche `feat/densite-dc1`)
État : fait, `main` fusionné (2d33df5a), fmt + clippy + 546 tests (data-model, sim-campaign, ai) verts, pytest codex et colonies verts. À fusionner dans `feat/densite`. `points_per_step` 140 → 70 (rules.json + défaut Rust) ; description de
rules.json ; codex (mouvement, saisons, déroute, agents) ; tests `campaign.rs`, `m2_free_movement.rs`,
`m4_path_plan.rs`, `c7a_retreat.rs`. Aucun code de règle changé : tout suit `points_per_step`.
- Saison : 105 km de plaine (70 en hiver), 140 sur route ; repli ami 140 km, refuge neutre 70 km ;
  agents 4 pas = 280 km (210 pour le prédicateur). ZdC 8, engagement 5, vision 30/20, recul 15 : inchangés.
- Traversées : `Embark` coûte toujours la saison entière (inchangé) ; pour l'IA/les agents l'arête
  maritime vaut 2 × 70 = 140, plafonnée à l'allocation (105) : toujours 1 tour. Sonde jetable
  (armées ayant franchi une arête maritime, 120 tours, graines 1-3) : avant Angleterre 7/10/10 ;
  après Angleterre 12/7/12 (+ Castille 1, Flandre 2). Pas de blocage.
- IA : `PLANNING_RANGE` 5 pas et `OFFENSIVE_RANGE` 4 pas suivent (même nombre de saisons, rayon
  géographique divisé par deux : 350 km). Aucun littéral 140/210/280 de campagne dans `core/crates`
  hors tests (les autres sont des mètres de bataille).
### Sondes (avant 140 → après 70)
- `march_range_probe` (Paris, tours été/hiver) : allocation 1460 → 730 pts (210 → 105 km).
  Orléans 1/1 → 1/2, Reims 1/1 → 2/2, Rouen 1/1 → 2/2, Calais 2/2 → 3/4, Tours 1/2 → 2/3,
  Dijon 2/2 → 3/4, Poitiers 2/2 → 3/4, Lyon 3/4 → 5/7, Bordeaux 3/4 → 5/7, Toulouse 3/5 → 6/9,
  Bayonne 4/5 → 7/10.
- `turn_perf 50 3 1` : moyenne 9,30 → 5,86 ms, p95 42,6 → 23,3, p99 105,9 → 63,8, max 271 → 256 ms.
- `century_probe 120 1 2 3` (30 ans) : guerre FR-EN 66 → 65 % ; batailles FR/EN par décennie
  19,0/39,7/20,0 → 16,7/17,3/23,0 ; prises 72/93/100 → 82/59/63 ; sièges engagés 101 → 71 (−30 %),
  réussis 36 → 25 % ; prises directes 96 → 98 ; révoltes /200 t. 9,4 → 0,6 ; banqueroutes 0,06 → 0,03 ;
  saisons d'intrusion 54 → 76. IA un peu moins offensive (sièges), pas passive.
### Points ouverts pour DC3
- Moins de sièges engagés et réussis (armées de secours/renforts plus lentes) : revoir
  `OFFENSIVE_RANGE`/`PLANNING_RANGE` et la durée des sièges une fois les colonies densifiées.
- Refuge neutre à 70 km : une armée anglaise battue en pleine France se débande plus souvent
  (les tests M2/C7a forcent l'ancien rayon de 140 km). Avec ~1 200 places, à revoir (garder 1 pas
  ou passer `neutral_radius_steps` à 2).
- Révoltes quasi nulles sur 30 ans (9,4 → 0,6) : à expliquer (armées de répression plus proches ?
  moins de dévastation ?).
- `game/scripts/ui/encyclopedia.gd` (agents) écrit « %d pas par saison » : pas de km, inchangé.
- `docs/design/2026-09-24-mouvement-libre.md` cite 210 km / × 140 km : spec datée, laissée telle quelle.

## DC4 — Affichage (worktree ../gp-dc4, branche feat/densite-dc4)
État : terminé, `feat/densite` et `main` fusionnés (smoke, settlements_render, da3, cv1 verts ; sonde
rejouée : 0 chevauchement de noms ni de maquettes, 8 maquettes masquées, 109 hameaux non posés
après les ancrages fins de `main`). Prêt pour fusion dans `feat/densite`.
- Sonde `game/tests/dc4_density_probe.gd` (fenêtrée ; `--center=2310.3,1657.9` = Lille, zone la
  plus dense : 14 places à moins de 40 unités ; `CENT_ANS_DATA_DIR` pour rejouer 570 places).
- Rangs (`settlement_markers.json`, règles dérivées des données, les listes d'ids restent en tête) :
  château rang 2 = fortification ≥ 3 ET poids ≥ 10 (les petits châteaux ajoutés restent rang 1) ;
  ville rang 2 aussi si fortification ≥ 2 ET poids ≥ 25. Vue Europe 68 places (inchangé), région
  264 (570 places : 245 ; 1 192 sans DC4 : 276), comté tout (1 192).
- Désencombrement des noms : grille spatiale (`LabelPlacer.SpatialGrid`), rectangles mesurés avec
  la police (l'ancienne estimation sous-évaluait la largeur : il y avait de vrais chevauchements),
  tri cité > ville > château > abbaye > village puis poids décroissant (`SettlementData`), aucun
  calcul au palier Europe ; noms dessinés au-dessus des marqueurs (priorité 4/3 contre 2).
- Maquettes : `_fit_model` répétable (taille d'origine gardée), aux positions de rendu (ancrages
  fins), ville emblématique non réduite (la voisine prend l'écart), plancher 0,55 → 0,4, maquette
  de faubourg masquée (`_absorb` : Saint-Maximin sous Trèves, Marmoutier sous Tours…) ; CV1 remet
  la maquette de croissance à pleine taille (réduction faite par `replace_model`, avant elle
  empilait une réduction périmée).
- Hameaux : non posés dans l'emprise d'une maquette de colonie (`on_settlement_model`, 60 cas).
- Picking : positions des marqueurs en cache, maquettes hors portée ignorées.
- ZG6 : choix des 16 finages par insertion au lieu d'un tri complet des 1 185 villes.

Mesures (1600×900, build debug ; temps d'image non comparables : charge machine 120-180 due aux
autres agents) :

| | 570, avant | 1 192, avant | 1 192, après |
|---|---|---|---|
| Noms qui se chevauchent (comté / près) | 4 / 6 | 7 / 13 | 0 / 0 |
| `declutter()` Europe / région / comté / près (µs) | 246 / 344 / 457 / 498 | 541 / 612 / 906 / 1 253 | 76 / 458 / 700 / 942 |
| Picking au comté (µs) | — | 4 091 | 230 |
| Marqueurs à l'écran Europe / région / comté | 37 / 72 / 95 | 37 / 81 / 218 | 37 / 78 / 218 |
| Maquettes voisines qui se recouvrent (> 20 %) | 14 | 34 | 0 (7 masquées, 174 réduites) |
| Hameaux dans l'emprise d'une colonie | 31 | 63 | 60, non posés |
| ZG6 : villes 1:1 chargées au max / tri streaming (µs, toutes les 10 images) | 1 / 811 | 5 / 1 946 | 5 / 617 |

Captures : `docs/img/dc4/` (avant570-comte, avant1192-comte, apres-comte, apres-region,
apres-europe).
Points ouverts : au comté, 35 paires de marqueurs se recouvrent à plus de moitié sur 218 (villages
serrés autour de Lille) — pas de désencombrement écran des marqueurs (le cahier veut tout au comté) ;
temps d'image à remesurer sur machine calme (`pb1_bench.gd`).
Build : `CARGO_TARGET_DIR` partagé entre worktrees : les rlib des crates du dépôt ont le même nom
d'un worktree à l'autre et cargo les croit à jour (mtime) même construites depuis un autre
worktree (constaté : dylib avec plafond 6 au lieu de 16, 126 avertissements « expected 1-6 »).
Parade : `find core/crates -name '*.rs' -exec touch {} +` avant `cargo build`, copier aussitôt.
