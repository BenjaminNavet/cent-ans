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
| DC1 | Mouvement : `points_per_step` 140 → 70 (rules.json + défaut Rust), vérifier traversées, horizon IA, agents, repli ; tests ; textes du codex et description de rules.json ; sondes `march_range_probe`, `turn_perf` | 1 | à lancer |
| DC2a | Colonies France nord, ouest, centre (19 prov., cible 12) | 1 | à lancer |
| DC2b | Colonies Aquitaine, Languedoc, France est, Provence-Alpes (22 prov., cible 11) | 1 | à lancer |
| DC2c | Colonies îles Britanniques (Angleterre cible 10 ; Galles, Irlande, Écosse 7) | 1 | à lancer |
| DC2d | Colonies Pays-Bas (11), Empire rhénan (9), reste de l'Empire et Scandinavie (7) | 1 | à lancer |
| DC2e | Colonies Ibérie et Italie (cible 7-8) | 1 | à lancer |
| DC3 | Régénération (`geo settlements`, `hamlets`, `anchors-fine`, `towns`), rangs de marqueurs, équilibrage économie/garnisons/entretien (Angleterre doit lever), IA et `turn_perf`, sim de 20 ans | 2 | après DC1 + DC2 |
| DC4 | Affichage : niveaux de détail des marqueurs, désencombrement des étiquettes (O(n²)), maquettes proches, captures | 2 | après DC2 |
| DC5 | Recette (orchestrateur) : build, smoke, cargo test, pytest, ff dans `main` | 3 | |

## Journal
- 26/09 : plan, squelette DC0.

## Prochaine étape
Lancer la vague 1 (DC1 + DC2a-e, 6 agents).

## DC1 — mouvement ralenti (worktree `../gp-dc1`, branche `feat/densite-dc1`)
État : fait, à fusionner. `points_per_step` 140 → 70 (rules.json + défaut Rust) ; description de
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
