# OMR R1 — coût de planification IA par tour (carte OM)

Branche `feat/omr-r1`, worktree `../gp-omr-r1`. Plan : `docs/wip/omr.md`.

## Objectif
Tour de jeu OM ≈ 1,07 s de planification IA (177 factions) contre 0,23 s sur l'ancienne carte ;
cible ≤ 0,45 s, décisions de l'IA inchangées (empreinte `turn_digest` identique à graine égale).

## Méthode
- Cible cargo partagée `core/target`, profils `r1` (dev, debug=0) et `r1rel` (release, debug=0).
- Mesure : `turn_perf 10 1 1` (release sans debug) ; égalité : `turn_digest` avant/après.
- Profil : `sample` sur `turn_perf`.

## État
- [x] Mesure de référence + empreinte de référence (`turn_digest 12 1 7`, 24 lignes)
- [x] Profil (`sample`) : attitude (faction_power + are_neighbors), GridPlanner::with_mode
  (trespassed_owner par colonie), agent_dijkstra, city_state, nearest_settlement, revenus.
- [x] C1 : `sim_campaign::planning_scope` — `CampaignState::planning_scope()` (garde qui emprunte
  l'état ; index enregistré sous l'adresse de l'état) : puissance par faction, voisins de chaque
  faction. `faction_power`, `are_neighbors`, `neighbour_factions` les lisent pendant la portée.
  Ouverte par `ai::plan_turn_in`. Test `sim-campaign/tests/omr_r1_planning_scope.rs`.
- [x] C2 : `GridPlanner::with_mode` : `trespassed_owner` une fois par contrôleur (mémo local).
- [x] C3 : `MovementGraph::index()` (data-model, `GraphIndex` paresseux : colonies en ordre
  d'id, arêtes en indices) ; `agents::AgentTable` (Dijkstra sur tableaux, même départage) ;
  `agent_find_path` s'arrête à la cible, `nearest_city` lit la table ; `agent_dijkstra_by_ids`
  garde l'ancien calcul (référence). Test `sim-campaign/tests/omr_r1_agent_paths.rs`.
- [x] C4 : `faction_income_effective` mémorisé dans la portée (`_walk` = calcul direct).
- [x] C5 : mémos de portée `diplomacy::rivals` (plan_alliances le demande pour la plupart des
  factions) et `controlled_provinces` (`_walk` = calcul direct) ; GridPlanner : verdict de
  passage par province (une recherche par colonie au lieu de deux + mémo), `stops` sans
  relecture de la colonie.
- [x] C6 : `GameData::nearest_settlement` sur une grille de colonies paresseuse
  (`data-model/src/settlement_grid.rs`, anneaux de cellules jusqu'à la borne ; repli sur le
  parcours si les tailles de `settlements`/`settlement_px` ont changé) : ancre de chaque armée
  en campagne, pour chaque faction. `march::nearest_settlement` la lit. Test
  `sim-campaign/tests/omr_r1_nearest.rs` (≈ 2 800 points, hors carte, milieux, NaN).
- [x] C7 : plan_economy — `Context::holds` sur la colonie déjà lue au lieu de `owns_settlement`
  (nouvelle recherche) dans les parcours.
- [x] ADR 0119 (portée de planification) — numéro choisi après 0117 (R3) et 0118 (R2).
- [x] fmt, clippy --workspace --all-targets -D warnings, `cargo test --workspace` : 1 228 ok,
  0 échec, 55 ignorés.
- [ ] Mesure finale

## Mesures
Machine partagée très chargée (charge ≈ 12 sur 14 cœurs) : le temps mur de `turn_perf` varie
du simple au double d'un passage à l'autre. Ajout de `turn_perf --sequential` : planification
sur le fil appelant, temps CPU du fil (clock_gettime THREAD_CPUTIME) — mesure du travail peu
sensible à la charge. A/B toujours base et branche l'un après l'autre.

| Version | turn_perf 10 1 1 (mur, moy./méd.) | --sequential CPU (moy./méd./p99) |
|---|---|---|
| base (a8a9c5bf9) | 5,49 / 2,70 ms (≈ 0,96 s par tour de jeu) | 10,02 / 6,32 / 51,6 ms |
| C1+C2 | bruit | 6,50 / 4,80 / 25,1 ms |
| C1-C4 | bruit (charge 95) | 5,71 / 4,43 / 23,8 ms (base au même moment : 9,73) |
| C1-C5 | bruit (charge 68) | 4,51 / 3,49 / 18,4 ms (base au même moment : 8,50 ; ×0,53) |
| C1-C7 | bruit (charge 58) | 3,57 / 2,55 / 17,7 ms (base au même moment : 8,26 ; ×0,43) |
| final, charge 22-28 | base 19,1 et 16,8 ; branche 13,1 et 4,57 ms (moy.) | base 8,23 / branche 3,44 ms (×0,42) |

Estimation par tour de jeu (machine calme) : base 0,96 s mesurée à 5,49 ms × 1 743 / 10 ;
× 0,42 (travail CPU) ≈ 0,40 s, sous la cible 0,45 s. Le temps mur `turn_perf 10 1 1` n'a pas pu
être confirmé sur machine calme (charge 20-95 pendant tout le lot) : à refaire à l'intégration.

Empreinte `turn_digest 12 1 7` : identique à la base après C1+C2, C1-C4, C1-C5, C1-C7.

Attention (cible partagée) : un worktree de base construit avec le même profil `r1rel` fait
passer ses crates pour à jour (dep-info vers l'autre chemin) ; base construite en `r1base`,
puis `cargo clean -p … --profile r1rel`.

## Prochaine étape
Lot terminé. À l'intégration : `turn_perf 10 1 1` (r1rel/release sans debug) sur machine
calme, base et branche l'une après l'autre. Pistes restantes si besoin de marge :
`recruitable_with_supply` (tous les types d'unités par site), `goods_map`/`free_supply`,
index des voisins reconstruit à chaque plan (≈ 4 %), `claim_stakes` dans `rivals_walk`.
