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
| DC3 | Régénération (`geo settlements`, `hamlets`, `anchors-fine`, `towns`), rangs de marqueurs, équilibrage économie/garnisons/entretien (Angleterre doit lever), IA et `turn_perf`, sim de 20 ans | 2 | fait (feat/densite-dc3, à fusionner) |
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

## DC3 — équilibrage de la carte densifiée (worktree `../gp-dc3`, branche `feat/densite-dc3`)
État : **fait** (voir « Reprise » plus bas), à fusionner dans `feat/densite` puis `main` (DC5).
Méthode : sondes construites depuis des worktrees détachés `../gp-dc3-main` (main) et
`../gp-dc3-dc1` (c3e5f17d, DC1 seul : 570 places + pas de 70 km), mêmes graines ; supprimés.
`century_probe` compte désormais : cités prises, provinces conquises en entier (durée depuis la
première place prise), dévastation, provinces occupées, mécontentement > 60, issues des replis ;
`REVOLT_TRACE=1` liste révoltes et provinces à plus de 70 de mécontentement.

### Économie d'ouverture (tour 0, toutes factions, `start_economy_probe`)
| | revenu | armée+garnisons | bâtiments | net |
|---|---|---|---|---|
| main | 143 662 | 69 909 | 22 707 | −9 261 |
| DC3 brut (1 192 places) | 143 199 | 80 133 | 27 621 | −25 288 |
| DC3 réglé | 143 199 | 69 706 | 22 081 | −9 321 |
Angleterre net 1 448 (main) → −1 657 (brut : elle ne lève plus d'armée, 1 unité de campagne au tour 15)
→ 1 374 (réglé). France 4 153 → −310 → 3 657.
Réglage (data/settlements/rules.json) : part de la couronne réduite pour que le total des places
secondaires reste celui d'avant la densification — garrison_upkeep_percent ville 15→8, château 25→15,
abbaye 10→5 ; building_upkeep_percent ville 40→25, château 50→30, abbaye 25→12, village 50→10.
Garnisons de départ inchangées.

### Données corrigées
19 nouveaux villages (DC2a) avaient un `bld_market` interdit aux villages, České Budějovice un
`bld_counting_house` sans foire (test eq2_balance) : retirés.
Tests adaptés : c6_agents (le héraut se recrute dans une cité ; les nouvelles abbayes passaient
avant), m2 `the_loser_falls_back_on_the_grid` (le point vide a changé : rayon neutre élargi dans le
cas du mur).

### Pause (26/09, demande du joueur)
Agent DC3 arrêté en cours de lot. Dernier travail commité : l'IA évalue recrutement et chantiers sur
une seule réserve de ressources par tour (`recruitable_with_supply`, `buildable_with_supply`) —
optimisation de fin de tour ; tests sim-campaign + ai et clippy verts, **pas encore mesurée**
(`turn_perf`). L'agent préparait des « variantes de partage de la réserve » (non commencées).

### Reprise (agent DC3 n° 2, 26/09) — lot terminé
État : fait. `main` (35783bcf, PB3 compris) fusionné (da2125c7) ; fmt, clippy -D warnings, 877 tests
Rust (espace de travail entier), pytest colonies/graphe/codex, dylib + import + smoke.gd verts.
Target cargo privé `gp-dc3/core/target` (les mesures d'avant la pause venaient du target partagé).
ADR 0082 : addendum DC3.

Réglages changés :
- IA : `PLANNING_RANGE` 5 → 10 pas (700 km comme avant DC1). La baisse des cités prises venait de
  DC1, pas de la densité (graines 1-6 : main 10,3/déc., DC1 4,9, DC3 5,5), surtout l'Angleterre en
  France (29 cités prises → 6 : elle ne voyait plus que la côte). Test m3 « Douvres » : seule la
  guerre franco-anglaise est gardée (l'horizon atteint Édimbourg, menacé par les Écossais).
- Effets de province : `province_effect_percent` (rules.json + schéma ; `province_building_effects`,
  `province_capacity`) : bâtiments des places secondaires à 50 %. Cause des révoltes disparues :
  DC1 raccourcit les occupations, puis la densité double l'apaisement des églises et abbayes
  (−10 → −21 en moyenne) et l'IA garde l'impôt haut (28 → 37 % des tours). « Une fois par sorte »
  essayé et écarté (biens des marchés divisés par deux, 16 % d'hommes en moins). Codex ordre public.
- Hameaux : `MIN_SETTLEMENT_DISTANCE_KM` 3 → 5 ; `geo hamlets` (2 999, aucun à moins de 5 km sauf 1
  à la limite) puis `geo anchors-fine` (le fichier de la branche n'ancrait que 569 colonies, écrasé
  par une fusion de main : 1 192 maintenant).
- Gardé : refuge neutre 1 pas (2 pas essayé : replis neutres 0,1 → 0,2, dispersions 1,25 → 1,1,
  sans effet mesurable).
- Perf de l'IA : vitesse de chantier une fois par place et `recruitable/buildable_with_supply`
  (faits aussi par PB3f dans main : version de main gardée), `nearest_settlement` en parcours
  fusionné, revenu brut une fois par tour d'IA, gouverneur une fois par province dans
  `faction_income_effective`, revenus paresseux (subsides, rançons, ordres de chevalerie).
  Résultats de sonde identiques avant/après.
- Sonde `century_probe` : `CAPTURE_TRACE=1`, `ARMY_TRACE=<faction>`, impôt haut, hommes en campagne,
  reprises aux rebelles.

### Mesures finales (`century_probe 120`, graines 1-12, même code que main)
| | main 35783bcf | DC1 c3e5f17d | DC3 final |
|---|---|---|---|
| Guerre FR-EN (% des tours) | 64,0 | 61,3 | 62,9 |
| Cités prises / décennie | 9,25 | 5,53 | 7,19 (−22 %) |
| Provinces conquises en entier / déc. | 3,75 | 2,58 | 3,14 (−16 %) |
| Durée d'une conquête (tours) | 5,03 | 4,21 | 4,88 (−3 %) |
| Sièges engagés / réussis | 82 / 46 % | 71 / 26 % | 58 / 50 % |
| Provinces occupées (% prov.-tours) | 0,82 | 0,42 | 0,48 |
| Mécontentement > 60 (‰ prov.-tours) | 7,2 | 5,6 | 6,8 |
| Révoltes / 200 tours | 5,7 | 2,5 | 3,2 |
| Impôt haut (% fac.-tours) / hommes en campagne | 28,5 / 27,0 k | — | 30,2 / 26,6 k |
| Banqueroutes / fac. / déc. | 0,07 | 0,05 | 0,06 |
| Replis neutres / débandades / dispersions | 0,5 / 0,75 / 0,6 | 0 / 1,4 / 4,0 | 0,1 / 0,75 / 1,25 |

DC1 est l'ancien code (avant la revue de code et PB3) : comparer DC3 à main.

`turn_perf 50 3 1` (release, 3 passes alternées, charge machine ~12) :
| | moyenne | p95 | p99 | max |
|---|---|---|---|---|
| DC1 | 2,94 ms | 10,2 | 14,9 | 20,7 |
| main (570 places, PB3) | 1,57 ms | 5,0 | 8,2 | 13,1 |
| DC3 final (1 192 places, PB3) | 2,66 ms | 9,6 | 15,2 (+2 % vs DC1) | 21,9 |

### Points ouverts
- Révoltes 3,2 contre 5,7 dans main : le mécontentement élevé est revenu (6,8 ‰ contre 7,2) mais les
  occupations restent plus courtes (0,48 % contre 0,82) : effet du pas de 70 km, pas de la densité.
- Sièges engagés −30 % mais mieux réussis (50 % contre 46 %) : les cités tombent à −22 %.
- Recherche : `research_points_per_turn` somme les bâtiments de toutes les places possédées
  (+102 abbayes à 0,25) : non mesuré.
- Hérésie : le compte des bâtiments religieux de la province n'est pas pondéré.
- IA vs main : +70 % de temps moyen (plus de places, horizon doublé), sous la cible de 50 ms.
- Rendu des hameaux (5 km) non revérifié dans Godot (`dc4_density_probe.gd`, fenêtré).
