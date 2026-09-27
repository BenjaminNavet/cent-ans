# CB-M2 — Aperçu du trajet et curseur contextuel (état)

Branche : `feat/cb-m2-path-hover`. Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md`
(section CB-M2, écart 1). Spec : `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md`.
Cible cargo privée : `CARGO_TARGET_DIR=<worktree>/core/target-cbm2`.

## État

- [x] Données : `data/rules/battle_hover.json` + `data/schemas/battle_hover_rules.schema.json` +
      `tools/tests/test_battle_hover_schema.py`.
- [x] Cœur, `plan_route` (`sim/pathing.rs`) : `route()` déplacé dans `pathing.rs` ; décisions
      partagées (`water_step`, `walks_straight_in_siege`, `climbs_straight`, `grid_step`,
      `leg_on_path`) ; `plan_route` enchaîne les mêmes décisions de point en point.
- [x] `preview.rs` : `preview_path`, `preview_group` (places du `Move` de groupe, barycentre
      au-delà de 6), `PreviewError`.
- [x] Tests `tests/cb_preview.rs` : ligne droite, pont, gué, siège, eau profonde, hachage d'état.
- [x] `hover.rs` : `hover_context` (table de la spec), `Compare` (9 lignes, avantage net) ; facteurs `horse_against_foot`, `pikes_against_horse`, `Unit::charge_points` extraits de `melee_damage` (mêmes opérations).
- [x] Pont `battle_sim_preview.rs` : `preview_path`, `preview_paths`, `hover_context` ; constantes
      `data_store_rules.rs`.
- [x] Godot : `battle_path_preview.gd`, `battle_cursor.gd`, survol carte HUD → contour (signal `card_hovered`),
      test `cb_m2_path_hover_test.gd` (vert). Reste : capture `cbm2_path_shot.gd` + sonde.

## Invariant aperçu = ordre (mesuré)

`route()` = premier point de `plan_route` recalculé à chaque pas depuis la position courante
(le plan l'impose : pas d'itinéraire stocké). Conséquences mesurées :
- ligne droite et pont : points de passage **identiques** (premières visites) ;
- gué : identiques sauf, dans le gué même, la berge visée à l'x de l'unité (≈ 2 m du point de
  sortie prévu) ;
- siège : premier point identique ; ensuite l'unité ne vise que des cases du même chemin A*
  (tirées de sa position réelle, donc plus de points intermédiaires) ou un recul hors d'un mur ;
  écart max. au tracé ≈ 13 m (poussée des maisons).

## Prochaine étape

Capture `cbm2_path_shot.gd` + sonde, puis vérifications complètes (cargo, pytest, godot).
