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

## Rendu
- `reachable_bubble.gdshader` : canal R = ce tour (voile parchemin, liseré doré, ombre d'encre),
  canal B = tour suivant (voile pâle, liseré fin estompé, sans ombre), `show_next`.
- `ArmyMovementBubble.set_suspended` : masquée pendant `end_turn_running` et `ai_replay.playing`
  (piloté par `ArmyMovementController._process`) ; `next_cell_count()`.
- Lord : `data/ui/campaign_map.json` `map.army_figure_scale` (schéma `campaign_map_ui.schema.json`),
  lu par `ArmyFigures.map_settings()`. Général agrandi et avancé (`LORD_ADVANCE`), porte-étendard
  à pied supprimé, hampe dans la main (`LORD_HAND`), ramenée au pied du marqueur avec le fondu
  des figurines au palier loin (`ArmyMarker.set_view` → `_follow_bearer`).
- Embuscade du joueur : `GeometryInstance3D.transparency` (`map.ambush_owner_transparency`) sur
  figurines, hampe, fleuron et drapeau. Icône de posture sur l'étendard : laissée à CV3-4 (pas
  d'icône d'encre embuscade / marche forcée / camp retranché à ce jour).

## État
- [x] Core `reach.rs` + `bounded_dijkstra_from`.
- [x] Tests Rust `tests/cv3_reach.rs` (4).
- [x] Pont RGB8 + champs.
- [x] Shader/bulle à deux tons, masquée en fin de tour et rejeu IA.
- [x] Lord TW + embuscade semi-transparente ; smoke (données de zone, lord, fantôme).
- [ ] Vérifs : cargo test complet, pytest, import Godot, smoke.
- [ ] Captures `docs/img/cv3/zone-*.png` (script `game/tests/cv3_zone_shot.gd`).

## Prochaine étape
Vérifications puis captures.
