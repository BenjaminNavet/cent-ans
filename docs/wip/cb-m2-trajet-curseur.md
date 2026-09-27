# CB-M2 — Aperçu du trajet et curseur contextuel (état)

Branche : `feat/cb-m2-path-hover`. Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md`
(section CB-M2, écart 1). Spec : `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md`.
Cible cargo privée : `CARGO_TARGET_DIR=<worktree>/core/target-cbm2`.

## État : terminé, en attente de relecture visuelle et de fusion (session principale)

- [x] Données : `data/rules/battle_hover.json` + `data/schemas/battle_hover_rules.schema.json` +
      `tools/tests/test_battle_hover_schema.py`. Valeurs d'aperçu exposées à RuleValues
      (`hover_preview_recompute_m`, `hover_preview_max_per_s`, `hover_preview_max_paths`).
- [x] Cœur, `plan_route` (`sim/pathing.rs`) : `route()` déplacé dans `pathing.rs` ; décisions
      partagées (`water_step`, `walks_straight_in_siege`, `climbs_straight`, `grid_step`,
      `leg_on_path`) ; `plan_route` enchaîne les mêmes décisions de point en point. `route()`
      calcule seul le premier point (pas d'allocation). Cache A* : lu seulement pour le départ
      propre du régiment, jamais écrit par l'aperçu.
- [x] `preview.rs` : `preview_path`, `preview_group` (mêmes places que le `Move` de groupe,
      y compris l'arrondi de `group_destinations` ; barycentre au-delà de 6), `PreviewError`.
- [x] `hover.rs` : `hover_context` (table de la spec), `Compare` (9 lignes, avantage net) ;
      facteurs `horse_against_foot`, `pikes_against_horse`, `Unit::charge_points` extraits de
      `melee_damage` (mêmes opérations, multiplications par 1 exactes).
- [x] Pont `battle_sim_preview.rs` : `preview_path`, `preview_paths`, `hover_context` (ids en
      `PackedInt32Array` : un `Array[int]` typé GDScript est refusé par godot-rust).
- [x] Godot : `battle_path_preview.gd` (pointillés, fantôme décale CB-M1, trajets et flèche
      d'attaque persistants), `battle_cursor.gd` (substituts), `battle_input.gd` (aperçu au clic
      droit maintenu, ordre refusé sans chemin), `battle_scene.gd` (curseur 1 fois/image au plus,
      par case de 2 m), `battle_hud.gd` (signal `card_hovered` → contour pâle).
- [x] Tests : `tests/cb_preview.rs` (9), `game/tests/cb_m2_path_hover_test.gd`.
- [x] Capture `game/tests/cbm2_path_shot.gd` → `docs/img/cb/cbm2-path.png` (non relue) ; sonde
      `--probe=<dossier>` + `game/tests/cbm2_path_probe.py` : tirets teintés 100 % (écart 103,2),
      côté 0 % (0,4), bruit 0,6.

## Invariant aperçu = ordre (mesuré)

`route()` = premier point de `plan_route` recalculé à chaque pas depuis la position courante
(le plan l'impose : pas d'itinéraire stocké). Conséquences mesurées :
- ligne droite et pont : points de passage **identiques** (premières visites) ;
- gué : identiques à 3 m près : dans le gué même, la berge visée à l'x de l'unité (≈ 2 m du
  point de sortie prévu) ;
- siège : premier point identique ; ensuite l'unité ne vise que des cases du même chemin A*
  (tirées de sa position réelle, donc plus de points intermédiaires) ou un recul hors d'un mur ;
  écart max. au tracé ≈ 13 m (poussée des maisons).

## Choix / écarts

- Aperçu pendant le clic droit maintenu (consigne perf), pas au simple survol.
- Groupe : refus de l'ordre seulement si aucun régiment n'a de chemin.
- Pas d'ordre `target_wall` automatique au clic droit sur un mur (curseur seulement).
- Survol par la carte : seulement les cartes du joueur (pas de cartes ennemies) ; la
  comparaison au survol d'un ennemi est calculée par le cœur mais son panneau relève de CB-M4.

## Prochaine étape

Relecture de `docs/img/cb/cbm2-path.png`, fusion ; icônes DA5 des 6 curseurs.
