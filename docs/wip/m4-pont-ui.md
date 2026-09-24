# M4 — pont et interface du mouvement libre (état)

Spec : `docs/design/2026-09-24-mouvement-libre.md` § 6 et § 7. Suit M2 (`docs/wip/m2-core-movement.md`).
Branche : `worktree-agent-a3602d6ab1a3f3f59` (partie de main `93d466c`).

## État

- [x] Cœur : `path_plan.rs` (`CampaignState::plan_path` → `PathPlan { points, turn_ends, cost, cost_this_turn }`), tests `tests/m4_path_plan.rs`.
- [x] Cœur : `Order::Attack` renvoie `OrderOutcome::Moved(MoveReport)` avec `StopReason::Engaged { army }`.
- [x] Pont `campaign_sim_movement.rs` : `get_reachable_area`, `find_path_points`, `move_army_to`, `move_army_to_settlement`, `attack_army`, `embark_army`, `submit_order_report`.
- [x] `get_army` : `position`, `settlement`, `movement_left`, `movement_max`, `planned_path`, `destination_point` (anciens champs gardés).
- [ ] Godot : `ArmyMovementController` (bulle, chemin, clic droit, animation, ZdC), marqueurs à la position libre.
- [ ] Tests Godot : `c5_settlements_ui_test` adapté, nouveau `m4_free_movement_ui_test`.
- [ ] Captures `docs/img/m4/`.
- [ ] Fusion de main.

## Prochaine étape

Scripts Godot `game/scripts/map/army_movement_*.gd` et branchement dans `campaign_map.gd`.
