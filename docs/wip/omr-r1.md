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
- [ ] autres points chauds (re-profil)
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

Empreinte `turn_digest 12 1 7` : identique à la base après C1+C2, après C1-C4.

Attention (cible partagée) : un worktree de base construit avec le même profil `r1rel` fait
passer ses crates pour à jour (dep-info vers l'autre chemin) ; base construite en `r1base`,
puis `cargo clean -p … --profile r1rel`.

## Prochaine étape
Re-profil de C1-C4 ; viser ≈ 4,7 ms CPU séquentiel (×0,47).
