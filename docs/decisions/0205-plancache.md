# ADR 0205 — Le cache de planification est un objet explicite

Statut : accepté (08/10, chantier SC, lot `plancache`).

## Contexte

La planification IA lit un état figé pendant tout le tour d'une faction et pose cent fois les mêmes questions : puissance militaire d'une faction (parcours des armées et garrisons), voisinage de deux factions (parcours des provinces), revenu (coûteux), rivaux. OMR R1 les mémorisait dans un registre global (`planning_scope`) indexé par l'adresse de l'état, ouvert par `CampaignState::planning_scope()` et consulté en cachette par `faction_power`, `are_neighbors`, `neighbour_factions`, `faction_income` et `rivals`. Une même fonction répondait donc différemment selon un état caché du processus (statique global, verrou, compteur d'entrées actives), et rien dans une signature ne disait si elle profitait du cache.

## Décision

- `sim_campaign::plan_cache::PlanCache<'a>` emprunte l'état et calcule paresseusement, une fois, ses réponses (puissance par faction, index des voisinages, revenu et rivaux mémorisés). Emprunter l'état rend toute réponse périmée impossible ; le cache est `Sync` (`OnceLock`, `Mutex`) pour les modes parallèles du planificateur.
- Les méthodes de `CampaignState` (`faction_power`, `are_neighbors`, `neighbour_factions`, `faction_income`, `diplomacy::rivals`) redeviennent de purs parcours, sans cache : c'est la voie des appels hors IA (résolution du tour, pont Godot, tests). Les doublons `*_walk` et `*_uncached` disparaissent.
- Le cache appartient au contexte de planification : `ai::campaign::plan_turn_in` en crée un, `Context` et `GridPlanner` le portent, et les planificateurs (`plan_diplomacy`, `plan_peace`, `plan_agents`, `ai_choose_coinage`, `ai_ransom_orders`, `ai_found_order`, `ai_may_trespass`, `plan_feudal`, `plan_treaties`, `plan_side_change`…) prennent un `&PlanCache` à la place de `&CampaignState` (l'état s'obtient par `cache.state()`).
- `Deal` (évaluation d'un traité) porte le cache : `negotiation::evaluate_treaty_with(cache, …)` ; `evaluate_treaty(state, …)` en est l'enveloppe pour les appelants hors IA. Idem `ransom_amount_with` et `PlanCache::attitude` (corps unique de `CampaignState::attitude`).
- Les hooks de politique féodale (`protection_score`, `host_score`), dont la signature est fixée par le cœur, créent un cache local.
- Sels de l'IA : `WOOL`, `DEFECTION`, `DYNASTIC` rejoignent `ai/src/salts.rs` (valeurs inchangées).
- `agent_dijkstra_by_ids`, référence des tests d'égalité, n'est compilée qu'avec la feature `test-support`.

## Conséquences

- Résultats identiques bit à bit : le cache renvoie les sommes des parcours (test `omr_r1_plan_cache`), et les tests de rejeu (ai, ct1, cv3, déterminisme) passent inchangés.
- Plus d'état global ni de verrou statique ; deux planifications du même état sur deux fils ne partagent rien qu'elles n'aient créé.
- Les appelants des planificateurs (tests compris) construisent `PlanCache::new(&state)` ; un cache jetable ne coûte rien tant qu'il n'est pas interrogé.
- Le revenu est calculé une fois par faction et par plan au lieu de deux fois par rançon (`ransom_amount` le demandait deux fois).
