# M2 — cœur du mouvement libre (état)

Spec : `docs/design/2026-09-24-mouvement-libre.md` § 3 et § 7. ADR : `docs/decisions/0010-free-army-movement.md`.
Branche : `m2-core-movement` (worktree `agent-acbe5c6448250fa88`).

## État

- [x] `data/movement/rules.json` + schéma : version M1 (fusionnée depuis main), aucune clé ajoutée.
- [x] data-model : `FreeMovementRules` (`entities/movement.rs`), `navgrid.rs` (grille `navgrid.png` ou repli `land_mask` ×2, raster `province_ids.png`, composantes connexes, cache par processus, chargement paresseux), `settlements_px.json` → `GameData::settlement_point`.
- [x] sim-campaign : `ArmyPosition`, `movement_left` (coûts de grille), `planned_path` (coins) + `destination`, `navigation.rs` (A* 8 voisins octile, lissage par ligne de vue, Dijkstra borné, `CellSet`), `march.rs` (exécution immédiate, ZdC, entrée dans une colonie, `Attack`, `Embark`, reprise des trajets).
- [x] Repli du perdant sur la grille (`movement::retreat_target`, `Retreat::{Friendly, Fallback, Rout}`).
- [x] `STATE_VERSION` 6, v5 refusée (`CampaignError::PreFreeMovementSave`).
- [x] IA (`ai_minimal`, `ai::campaign`) adaptée mécaniquement : planification sur le graphe depuis l'« ancre » (colonie de l'armée ou la plus proche), ordres `MoveArmy` vers une colonie.
- [x] Pont : compile, getters dérivés (voir plus bas).
- [x] Tests § 7 cœur : `tests/m2_free_movement.rs` + adaptations (`campaign.rs`, `c7a_retreat.rs`, `m7.rs`…).
- [x] Fusion de main (d84a7bf), fmt, clippy -D warnings, `cargo test` complet vert, `build.sh`, smoke Godot vert (une assertion de `smoke.gd` adaptée : l'armée marche aussitôt).
- [ ] `c5_settlements_ui_test.gd` : 2 échecs attendus (chemin vers Wissant vide car la marche est immédiate ; l'armée a quitté Paris avant l'ordre de garnison) — à adapter en M4 (spec § 7).

## API pour M3 / M4

- Ordres : `Order::MoveArmy { army, target }` (`target` = colonie, province, point `{x, y}` en pixels carte 4096, ou ancien `path` dont on prend le dernier élément ; alias JSON `path`), `Order::Attack { army, target_army }`, `Order::Embark { army, to_port }`.
- `CampaignState::submit_order_outcome` / `apply_order_outcome` → `OrderOutcome::Moved(MoveReport { walked, cost, stop: StopReason, planned_path })`.
- Requêtes : `find_path(data, army, [x, y]) -> Option<GridPath { cells, waypoints, cost }>`, `reachable_area(data, army) -> Vec<(Cell, u32)>`, `reachable(data, army)` (colonies atteignables), `army_point`, `army_province`, `army_anchor`, `army_grid_allowance`, `armies_near`, `armies_together`.
- `march::continue_marches(state, data, Some(faction), events)` : reprise des `planned_path`.
- Les événements des actions immédiates vont dans `pending_events` (journal du tour suivant).

## Signatures changées (pont, autres lots)

- `Army.location`/`movement_points`/`path` → `position`/`movement_left`/`planned_path` + `destination`.
- `find_path` prend un point et renvoie `GridPath` ; `find_path_to_province` supprimé ; `movement::validate_path` supprimé.
- `armies_in`, `hostile_armies_in`, `friendly_armies_in`, `battle_coalition` prennent `data`. `movement::settlement_coalition` pour les sièges.
- `movement::fight`, `auto_fight`, `apply_battle_result` : plus d'`attacker_origin` ; `BattleRequest.attacker_origin` supprimé ; `BattleRequest.location` = colonie la plus proche pour une bataille en campagne.
- `SiegeState.started_turn` (un siège ouvert pendant le tour ne progresse qu'au tour suivant).
- Pont (compatibilité jusqu'à M4) : `get_army` renvoie `location` = colonie ou la plus proche, `movement_points` = `movement_left` (unités de grille), `path` = destination d'un trajet multi-tours ; `find_path` renvoie `[cible]` si atteignable ; `find_path_provinces` suit les cases.

## Ce que M3 doit changer

- `turn.rs` : tour séquentiel propre (le mien fait seulement : pour chaque IA par id, `continue_marches` puis ordres ; reprise des trajets du joueur après la remise à neuf des points).
- IA sur la grille : attaque d'armées dans la bulle (`Order::Attack`), évitement des ZdC plus fortes, embarquements (`Embark`, aujourd'hui l'IA ne traverse plus la mer : `crosses_sea` exclut déjà ces cibles), cache des chemins.
- Performance mesurée (release) : Paris-Bayonne 11 ms, bulle d'été 15 ms ; étiquetage des composantes 25 ms une fois par processus.

## Limites connues

- Les batailles en attente du joueur (M7) ne bloquent plus les armées par position : elles sont immobilisées (`movement_left = 0`) jusqu'à la résolution.
- ZdC testée au passage de chaque case, pas le long du segment.
- `Retreat::Neutral` du C7a remplacé par `Fallback` (recul de `retreat_fallback_km`).
