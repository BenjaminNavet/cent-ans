# CB1 — Formation au glisser et verrouillage de groupe (état)

Branche : `feat/cb1-drag-formation` (depuis `main` b985d752). Plan :
`docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` (section CB1, écarts 5 et 6).
Cible cargo privée : `CARGO_TARGET_DIR=<worktree>/core/target-cb1`.

## État : terminé, en attente de relecture visuelle et de fusion (session principale)

- [x] Données : `data/rules/formation_width.json` (bornes de rangs par classe : piquiers 4-16,
      fantassins 2-16, tireurs 2-8, cavaliers 1-6, engins 1 ; intervalle de groupe 10 m) +
      schéma `formation_width_rules.schema.json` + `tools/tests/test_formation_width_schema.py`.
- [x] Cœur : `src/formation_width.rs` (`FormationWidthRules`, `line_shape`, `split_widths`,
      `Unit::files_for_width`/`extent_for_width`/`set_width`) ; `Unit.line_files`,
      `group_tag`, `match_speed` (omis du JSON par défaut) ; `ranks_files` en Ligne suit
      `line_files` (rangs bornés, effectif conservé) ; `Command::Move` + `width`/`match_speed`/
      `group_tag` (omis du JSON par défaut) ; `QueuedOrder::Move` idem (largeur appliquée au
      départ de l'ordre) ; `sim/width.rs` (parts au prorata des effectifs, allure du plus lent
      sur l'allure en terrain ouvert, étiquette automatique `AUTO_GROUP_TAG | plus petit id`) ;
      `group_destinations_with` (largeurs individuelles) ; `state_digest` seulement si fixé ;
      ordre Formation = retour à la profondeur par défaut ; halte et attaque quittent le groupe ;
      `deploy_unit_width` + `ReplayAction::DeployUnit.width`.
- [x] Pont : `deploy_unit(..., width = 0)`, `preview_paths(..., width = 0)`,
      `preview_paths_queued(..., width = 0)` (jambes avec `width`/`depth` d'arrivée),
      `formation_extent(id, width)`, `get_units` : `line_files`, `group_tag`, `match_speed`.
- [x] Godot : `formation_drag.gd` (fonctions pures : `drag_line`, `split_widths`,
      `lateral_order`, `lock_shape`, `rigid_places`) ; `battle_input.gd` (glisser-droit = `width`,
      clic simple sans ; Maj + glisser = en file avec largeur ; Ctrl/Cmd+G = verrou ;
      `locked_orders` : `move` individuels, `match_speed`, même `group_tag`) ;
      `battle_path_preview.gd` (fantômes à la taille d'arrivée, `compute_places` pour un groupe
      verrouillé, `show_ghosts` en déploiement) ; `deployment_controller.gd` (`plan` : prorata,
      ordre gauche-droite, largeur retenue par le cœur ; `deploy_unit` avec largeur) ;
      `battle_groups.gd` (verrous, sortie d'une unité en déroute) ; `unit_card.gd` (cadenas
      dessiné en code) ; aide F1.
- [x] Tests : `sim-battle/tests/cb1_width.rs` (12), `game/tests/cb1_drag_formation_test.gd`,
      capture `game/tests/cb1_drag_shot.gd` (`--probe` OK : fantômes 48 × 6 contre 22 × 6,
      cadenas 2).
- [x] Golden CB0 : la commande 1 (glisser-droit) gagne `width` (seule retouche).

## Vérifications (09-27)

`cargo fmt`, `cargo clippy --workspace --all-targets -D warnings` propres ; `cargo test --workspace`
1008 réussis, 0 échec ; release `ep13_replay` 8/8, `b6` 12/12 ; pytest 797 réussis ; Godot
`smoke.gd`, `cb1_drag_formation_test`, `cb0_input_equivalence_test`, `cb_m1_outline_test`,
`cb_m2_path_hover_test`, `cb_m3_queue_test`, `cb_m4_range_compare_test`,
`cb1_drag_shot.gd --probe` : code 0, aucune « SCRIPT ERROR ».

## Choix / écarts

- Une largeur fait passer le régiment en Ligne (glisser = ligne, comme TW) ; un ordre Formation
  rend la profondeur par défaut.
- La répartition au prorata est une règle du cœur (un seul `Move` de groupe avec la largeur
  totale, `group_gap_m` retranché) ; l'aperçu reçoit la largeur et la profondeur d'arrivée de
  chaque régiment. `FormationDrag.split_widths` (même calcul) sert au seul déploiement, dont le
  placement reste côté Godot (intervalle 6 m existant).
- `match_speed` plafonne à l'allure en terrain ouvert du plus lent (vitesse, course, formation,
  fatigue) ; le terrain continue de ralentir chacun.
- Groupe verrouillé : clic simple = translation, orientation du verrouillage gardée ; glisser =
  translation + rotation, sans largeur.
- L'échantillon de rejeu EP13 diverge déjà sur `main` à 1 min 52 s (tick 1120, changements de
  règles antérieurs) : le test vérifie que CB1 ne le fait pas diverger plus tôt.

## Capture à produire (session principale)

`godot --path game --resolution 1600x900 --script res://tests/cb1_drag_shot.gd -- --out=docs/img/cb/cb1-drag.png`
(non exécutée ici, non relue).

## Prochaine étape

Relecture de la capture, fusion ; icône DA5 du cadenas ; addendum ADR 0095 (fichier
`formation_width.json`, largeur = Ligne, allure en terrain ouvert).
