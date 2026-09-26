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
| DC1 | Mouvement : `points_per_step` 140 → 70 (rules.json + défaut Rust), vérifier traversées, horizon IA, agents, repli ; tests ; textes du codex et description de rules.json ; sondes `march_range_probe`, `turn_perf` | 1 | fait, fusionné |
| DC2a | Colonies France nord, ouest, centre (19 prov., cible 12) | 1 | fait, fusionné |
| DC2b | Colonies Aquitaine, Languedoc, France est, Provence-Alpes (22 prov., cible 11) | 1 | fait, fusionné |
| DC2c | Colonies îles Britanniques (Angleterre cible 10 ; Galles, Irlande, Écosse 7) | 1 | fait, fusionné |
| DC2d | Colonies Pays-Bas (11), Empire rhénan (9), reste de l'Empire et Scandinavie (7) | 1 | fait, fusionné |
| DC2e | Colonies Ibérie et Italie (cible 7-8) | 1 | fait, fusionné |
| DC3 | Régénération (`geo settlements`, `hamlets`, `anchors-fine`, `towns`), rangs de marqueurs, équilibrage économie/garnisons/entretien (Angleterre doit lever), IA et `turn_perf`, sim de 20 ans | 2 | fait, fusionné |
| DC4 | Affichage : niveaux de détail des marqueurs, désencombrement des étiquettes (O(n²)), maquettes proches, captures | 2 | fait, fusionné |
| DC5 | Recette (orchestrateur) : build, smoke, cargo test, pytest, ff dans `main` | 3 | fait |

## Journal
- 26/09 : plan, squelette DC0 (6f74c214) ; vague 1 lancée (DC1 opus, DC2a-e sonnet, worktrees ../gp-dc1, ../gp-dc2a-e).
- 26/09 : DC2c : 105 colonies ajoutées (îles Britanniques ; village 43, abbey 27, town 18, castle 17). Fusionné.
- 26/09 : DC2d : 139 colonies ajoutées (village 52, town 55, abbey 24, castle 8) dans les 33
  provinces Pays-Bas/Empire/Scandinavie ; `settlement_check` et `pytest test_settlements_schema.py`
  passent (134 passed, aucun id en double, aucune paire <3 km introduite).
- 26/09 : DC2a : 128 colonies ajoutées (31 town, 15 castle, 15 abbey, 67 village) sur 19 provinces de
  France nord/ouest/centre ; `settlement_check` et `test_settlements_schema.py` passent.
- 26/09 : DC2e : 118 colonies ajoutées (52 town, 44 village, 12 abbey, 10 castle) sur 34 provinces
  d'Ibérie et d'Italie (Portugal, Aragon, Ibérie nord/centre/sud, Italie nord/centre/sud) ; Mallorca et
  Roussillon déjà à leur cible n'ont reçu aucun ajout côté Roussillon (7/7), 3 côté Mallorca (6/6).
  `settlement_check` et `pytest tools/tests/test_settlements_schema.py` verts sur les 35 provinces.
- 26/09 : DC2 fusionné : 1 192 colonies (city 132, town 407, village 291, abbey 189, castle 173). Régénéré : graphe (2 987 arêtes), positions, fine_anchors, hameaux, towns_1340 (1 185). À surveiller DC3 : +66 châteaux (entretien) ; côte est du Sussex dans le polygone du Kent. DC4 lancé.

- 26/09 : DC1 fusionné (main inclus, fine_anchors/towns régénérés sur le nouveau relief, 159aff80) ; DC3 lancé ; DC4 fusionné (848fff43 : 0 chevauchement d'étiquettes, désencombrement en grille, captures docs/img/dc4/).

## DC5 — recette (26/09, worktree `../gp-densite`)
État : recette verte sur `feat/densite` (4bc56f38 + ce commit) ; reste le ff dans `main` (orchestrateur)
puis la suppression des worktrees `../gp-densite`, `../gp-dc3`.
- Fusion `feat/densite-dc3` (5d44a9d5) : `settlement_layer.gd` combine DC4 et SZ4b. Les échelles se
  composent : la réduction DC4 porte sur la maquette enfant (`model.scale *= fit`, `_model_radius` =
  rayon d'origine × fit), l'échelle SZ4b sur le porteur (`holder.scale = model_scale(i)`) ; rayon
  affiché = rayon d'origine × fit × `model_scale(i)`. Le clic garde le filtre DC4 (portée de
  maquette, absorbées ignorées) et multiplie rayon/hauteur par `model_scale(i)`. `_real_ratio` =
  rayon réel / rayon après fit : de près la maquette converge vers l'emprise réelle quel que soit fit.
  `fine_anchors.json` de DC3 (1 192 colonies), `towns_1340.json` régénéré (1 185 villes).
- `data/map/navgrid.png` était périmé (test_navgrid) : `geo navgrid` relancé (96d628ba).
- `main` fusionné (4bc56f38, DA2 : aucun changement Rust ni carte).
- Résultats (target privé `gp-densite/core/target`) : fmt, clippy -D warnings, 877 tests Rust (espace
  de travail) ; dylib copiée, aucun avertissement « expected 1-6 » ; import Godot ; smoke,
  settlements_render (1 192 colonies, 2 999 hameaux), da3_markers, cv1_campaign_life, sz4_prop_scale,
  sz4b_colonies_forests, da2_living_portrait : OK ; pytest 764 passed ; ruff : 10 erreurs
  préexistantes dans `tools/blender_scripts/` (identiques dans main).
- Points ouverts : `geo navgrid` signale 5 nouvelles « îles sans port » (Hollande/Frise : Delft,
  Den Haag, Egmond, Haarlem, Leiden ; Appingedam ; Dokkum ; Gouda ; Leeuwarden — Pomposa et Teylingen
  l'étaient déjà dans main) : places sans accès terrestre ni port sur la grille, à corriger (port ou
  passage) ; `_absorb` et `_fit_model` raisonnent sur la taille de carte (échelle 1), pas sur la
  taille réduite de près (masquage un peu conservateur au palier vallée).

## État final (26/09)
Chantier terminé et fusionné dans `main` (ff a661314f). Correctif après recette : Haarlem, Gouda,
Dokkum, Leeuwarden et Appingedam deviennent des ports (sinon îles sans accès sur la navgrid).
Worktrees et branches supprimés. Dylib du checkout principal reconstruite, smoke vert.

## DC6 — suites (lancé 26/09, accord du joueur « ok pour la suite »)
| Lot | Contenu | Worktree | État |
|---|---|---|---|
| DC6a | Données : Pomposa et Teylingen accessibles ; audit des ~40 colonies recalées (coordonnées fausses vs frontière approximative) ; erreurs signalées par DC2 (Ranverso, Schloss Tirol, Skanör, Marienweerd, Bergerac, paires < 3 km) | ../gp-dc6a (feat/dc6-a) | lancé |
| DC6b | Équilibrage : recherche des abbayes et bâtiments religieux contre l'hérésie pondérés comme province_effect_percent ; révoltes (3,2 vs 5,7) | ../gp-dc6b (feat/dc6-b) | lancé |
| DC6c | Affichage : masquage/réduction des maquettes voisines à l'échelle réelle en vue rapprochée | ../gp-dc6c (feat/dc6-c) | lancé |
Frontière Sussex/Kent (polygones de provinces) : hors DC6, changement de géométrie lourd.

## Suites possibles
- Pomposa et Teylingen : îles sans port sur la navgrid (déjà dans main avant DC).
- Masquage de près des maquettes voisines calculé à l'échelle de carte (un peu trop large).
- Points de recherche des +102 abbayes et compte des bâtiments religieux contre l'hérésie non pondérés.
- Révoltes 3,2 / 200 tours contre 5,7 avant (occupations plus courtes avec le pas de 70 km).
- ~40 colonies recalées dans leur province (frontières approximatives, ex. côte est du Sussex dans le Kent).

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

## DC6c — réduction et masquage des maquettes à l'échelle effective (worktree `../gp-dc6c`, branche `feat/dc6-c`)
État : **fait**, `main` fusionné (696713bc), prêt pour ff. Tests verts : smoke, settlements_render,
da3_markers, cv1_campaign_life, sz4_prop_scale, sz4b_colonies_forests, dc6c_fit_scale (nouveau).
- `SettlementFit` (`game/scripts/map/settlement_fit.gd`, fonctions pures) : la place laissée par
  les voisines (`_room`, part de l'écart au prorata des poids, ou écart − rayon d'une ville
  emblématique) ne dépend pas des rayons ; la réduction est recalculée sur le rayon rétréci par
  SZ4b : rayon affiché = rayon d'origine × sigma × `fit_factor(room, rayon d'origine × sigma)`.
  `_model_radius` garde le rayon réduit à la taille de carte (hameaux, végétation, effets CV1 :
  inchangés) ; l'échelle du porteur `_model_scale` = `zoom_scale(...)` (1 au loin, ≤ 1) porte
  la différence, donc picking, anneau et hauteurs d'étiquettes suivent sans changement.
- Masquage : paires candidates (rayons de carte, sur-ensemble valable à toute échelle : 21 paires)
  calculées une fois (ancrages fins, `replace_model` CV1 : paires de la maquette seulement),
  masquage réévalué à chaque pas d'échelle SZ4b (`rewrite_step`), 0,13 ms. Nouvelle règle
  `ABSORB_OVERLAP` : masquée aussi si le recouvrement dépasse 20 % du plus petit rayon (réduction
  bloquée au plancher : Marmoutier sous Tours restait à 0,71 contre 0,51 de place à d = 14).
- **Régression trouvée dans `main`** : `_update_model_visibility` (villes 1:1, SZ4b) remettait
  `visible = true` sur toutes les maquettes, annulant le masquage DC4 dès la première mise à jour
  des villes : dans `main`, 0 maquette masquée et 7 paires qui se recouvrent en vue comté
  (Trèves/Saint-Maximin, Tours/Marmoutier, Le Mans/Épau…). Corrigé : `_apply_model_visibility`
  combine masquage et ville 1:1.
- Sonde `dc4_density_probe.gd` : statistiques de maquettes par vue (`models_shown` : masquées,
  réduites, paires affichées > 20 %), vues 20 (`rapproche`) et 12 (`vallee-haut`) nommées.
  Lille (`--center=2310.3,1657.9`, vues 1400,550,250,120,20,12, `--no-valley`) :

| | main | DC6c |
|---|---|---|
| Europe / région / comté / 120 / 20 : masquées, réduites, paires > 20 % | 0, 173, 7 | 7, 166, 0 |
| Distance 12 : masquées, réduites, paires > 20 % | 0, 173, 2 | 1, 7, 0 |
| Noms qui se chevauchent (toutes vues) | 0 | 0 |

  Test headless (toute la carte) : masquées/réduites 7/166 de 1000 à 20, 4/35 à 14, 0/2 à 10, 0/1 à
  7 ; 0 paire > 20 % à toutes les distances ; vue stratégique : échelle 1 pour toutes les maquettes.
- Captures `docs/img/dc6c/` (960 px) : `lille-*` et `lemans-*` (L'Épau), `avant`/`apres`, `comte`
  (250 : marqueurs, identiques) et `pres` (distance 12 : L'Épau de nouveau affichée à côté du Mans).
- Points ouverts : coût de `_place_model` pour 1 192 maquettes par pas d'échelle ~3,5 ms en debug
  sous charge (préexistant, SZ4b ; le masquage n'ajoute que 0,13 ms) ; les maquettes masquées
  à mi-zoom réapparaissent d'un coup (pas de fondu).

## DC6b — sommes non pondérées et révoltes (worktree `../gp-dc6b`, branche `feat/dc6-b`)
État : en cours. Référence d'avant densification : worktree détaché `../gp-dc6b-ref` sur
**9c107692** (dernier `main` avant la fusion DC : même code que 35783bcf de DC3 + DA2 ;
89bc960a, proposé, précède PB3 et la revue de code, donc un autre code d'IA). Sonde :
`DC6_TRACE=1 century_probe` (points de recherche par faction aux tours 1/40/120/200, hérésie
de chaque tour).
- Recherche : pondérée par un réglage dédié `research_percent` (rules.json, 70 % hors cité) ;
  50 % (= `province_effect_percent`) essayé : −6 % au tour 1 mais −16 % au tour 40 (France −21 %).
- Hérésie : compte des bâtiments religieux pondéré par `province_effect_percent` (50 %)
  (`religion::weighted_religious_buildings`) : province-tours hérétiques 17,7 → 9,7 (DC) → 17,3.
- Recherche à 70 % remesurée : +5 % au tour 1, −6 % au tour 40 (France +7/−4, Angleterre 0/−4).
- Révoltes : l'explication DC3 ne tient pas (occupations plus longues, pas plus courtes ; la baisse
  porte sur les provinces non occupées, surtout Luxembourg, Alentejo, Savoie ; garnison par province
  690 → 1 032 hommes). Variantes `garrison_relief_per_100_men` / `_max` (population.json) : très
  bruitées sur 12 graines ; relance sur 24 graines (13-24) en cours.
