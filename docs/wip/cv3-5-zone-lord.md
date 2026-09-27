# CV3-5 — Zone atteignable à deux tons et lord « à l'échelle TW »

Branche : `feat/cv3-5-zone-lord`. Spec : `docs/design/2026-09-27-campagne-vivante.md` § 4.
Orchestration : `docs/wip/cv3-campagne-vivante.md`.

## Contrat
- Core : `CampaignState::reachable_cells(data, army) -> ReachableCells { this_turn: Vec<(Cell, u32)>,
  next_turn: Vec<Cell>, budget, next_budget }` (`sim-campaign/src/reach.rs`). Tour 1 =
  `movement_left` (bonus de marche forcée déjà dedans) ; tour 2 = `army_base_grid_allowance` (la
  marche forcée expire au début du tour). Mêmes coûts que la marche, colonies hostiles exclues, ZdC
  des ennemis **vus** (pas de fuite d'embuscade) : une case de ZdC est entrée mais pas quittée ; un
  tour qui commence dans une ZdC ignore cet ennemi (comme `march::simulate`).
- `navigation::bounded_dijkstra_from` : Dijkstra borné multi-sources avec cases d'arrêt.
- Pont : `get_reachable_area` garde ses champs ; `image` passe de RG8 à RGB8 (B = 255 dans la
  zone « tour suivant », tour 1 compris), cadre couvrant les deux anneaux ; nouveaux champs
  `next_cells` (cases du seul 2e anneau) et `next_budget`.

## État
- [x] Core `reach.rs` + `bounded_dijkstra_from` (compile).
- [ ] Tests Rust (`tests/cv3_reach.rs`).
- [ ] Pont RGB8 + champs.
- [ ] Shader/bulle à deux tons, masquée en fin de tour et rejeu IA.
- [ ] Lord TW (`data/ui/campaign_map.json` `map.army_figure_scale`).
- [ ] Embuscade semi-transparente pour le propriétaire (icône de posture → CV3-4).
- [ ] Captures `docs/img/cv3/zone-*.png`.

## Prochaine étape
Tests Rust du second anneau.
