# M4 — pont et interface du mouvement libre (état)

Spec : `docs/design/2026-09-24-mouvement-libre.md` § 6 et § 7. Suit M2 (`docs/wip/m2-core-movement.md`).
Branche : `worktree-agent-a3602d6ab1a3f3f59` (partie de main `93d466c`, main `45f7ad5` fusionné).

## État : terminé (en attente de fusion par l'orchestrateur)

- [x] Cœur : `path_plan.rs` (`CampaignState::plan_path` → `PathPlan { points, turn_ends, cost, cost_this_turn }`, fonction pure), tests `tests/m4_path_plan.rs` (1 tour, 3 tours et arrêt identique à la vraie marche, cible inatteignable, armée sans points, attaque).
- [x] Cœur : `Order::Attack` renvoie `OrderOutcome::Moved(MoveReport)` avec `StopReason::Engaged { army }` (trajet d'approche pour l'animation).
- [x] Pont `campaign_sim_movement.rs` : `get_reachable_area` (masque RG8 recadré, trouées de 1-2 cases comblées pour l'affichage), `find_path_points`, `get_movement_rules`, `move_army_to`, `move_army_to_settlement`, `attack_army`, `embark_army`, `submit_order_report` (rapport : `walked` départ compris, `stop`, `stop_settlement`, `stop_army`, `planned_path`, `position`, `settlement`, `movement_left`, `events`), `debug_place_army` (tests/captures).
- [x] `get_army` : `position`, `settlement`, `movement_left`, `movement_max`, `planned_path`, `destination_point` ; anciens champs (`location`, `movement_points`, `path`…) gardés.
- [x] Godot : `ArmyMovementController` (bulle, chemin deux couleurs au survol, clic droit sol/armée/colonie/port, animation, cercle de ZdC), `ArmyMovementBubble` + `reachable_bubble.gdshader`, `ArmyMovementPath`. Marqueurs à la position libre (`army_markers.gd`), état « en marche » avec `planned_path`.
- [x] Branchements : `campaign_map.gd` (création, sélection, désélection, survol de province coupé, masque de provinces coupé, `--stage=movement|movement_near`), `settlement_controller.gd` (anneaux et aperçu C5 coupés si mouvement libre), `tutorial_controller.gd` (étape « marche » : position libre prise en compte).
- [x] Tests Godot : `m4_free_movement_ui_test.gd` (bulle, chemin deux couleurs, clic au sol → position, animation, ZdC, attaque), `c5_settlements_ui_test.gd` adapté (garnison avant la marche, colonie → stationnement), `smoke.gd` adapté (aperçu du mouvement libre), `settlements_render_test` OK.
- [x] Captures `docs/img/m4/bulle-chemin.png`, `bord-de-bulle.png` ; doc `docs/godot-map.md` § « Mouvement libre des armées (lot M4) ».
- [x] Fusion de main.

## Décisions

- Masque recadré à la bulle (une case par texel, ~330² en été) plutôt qu'une image 2048² : transfert et mipmaps négligeables.
- Maillage de la bulle posé sur le relief (≤ 96² quads) + shader (9 prises, seuil 0,5, liseré à largeur écran constante) plutôt qu'un décalque : contour anticrénelé et voile paramétrables.
- Trouées de 1-2 cases comblées dans le pont (affichage seulement) : sinon chaque fleuve raye la bulle.
- L'aperçu ne simule pas les zones de contrôle (trajet voulu) ; la marche réelle peut s'arrêter avant.
- `map.reachable` (provinces) reste calculé pour le tutoriel et le smoke.
- Clic droit sur une colonie injoignable par terre depuis un port : essai d'`embark_army` (le cœur valide).

## Limites

- Pas d'animation des armées IA pendant le tour IA (le rapport de saison suffit en v1, spec).
- Pas de fondu de la bulle ; recalcul complet à chaque `refresh_all` (≈ 15 ms de Dijkstra en release).
- Pas d'indicateur d'arrêt en ZdC sur l'aperçu.

## Prochaine étape

Fusion par l'orchestrateur ; M5 (vision, équilibrage).
