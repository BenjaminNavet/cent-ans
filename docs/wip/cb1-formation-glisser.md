# CB1 — Formation au glisser et verrouillage de groupe (état)

Branche : `feat/cb1-drag-formation` (depuis `main` b985d752). Plan :
`docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` (section CB1, écarts 5 et 6).
Cible cargo privée : `CARGO_TARGET_DIR=<worktree>/core/target-cb1`.

## État : cœur et pont en place, Godot à faire

- [x] Données : `data/rules/formation_width.json` (bornes de rangs par classe : piquiers 4-16,
      fantassins 2-16, tireurs 2-8, cavaliers 1-6, engins 1 ; intervalle de groupe 10 m) +
      schéma `formation_width_rules.schema.json` + `tools/tests/test_formation_width_schema.py`.
- [x] Cœur : `src/formation_width.rs` (`FormationWidthRules`, `line_shape`, `split_widths`,
      `Unit::files_for_width`/`extent_for_width`/`set_width`) ; `Unit.line_files`,
      `group_tag`, `match_speed` (omis du JSON par défaut) ; `ranks_files` en Ligne suit
      `line_files` ; `Command::Move` + `width`/`match_speed`/`group_tag` (omis du JSON par
      défaut) ; `QueuedOrder::Move` idem ; `sim/width.rs` (parts au prorata des effectifs,
      allure du plus lent, étiquette automatique `AUTO_GROUP_TAG | plus petit id`) ;
      `group_destinations_with` (largeurs individuelles) ; `state_digest` seulement si fixé ;
      ordre Formation = retour à la profondeur par défaut ; `deploy_unit_width` +
      `ReplayAction::DeployUnit.width`.
- [x] Pont : `deploy_unit(..., width = 0)`, `preview_paths(..., width = 0)`,
      `preview_paths_queued(..., width = 0)` (jambes avec `width`/`depth` d'arrivée),
      `formation_extent(id, width)`, `get_units` : `line_files`, `group_tag`, `match_speed`.
- [ ] Tests cœur `tests/cb1_width.rs`.
- [ ] Godot : fantôme au glisser, verrouillage Ctrl/Cmd+G, cadenas, déploiement.
- [ ] Test Godot, script de capture (`--probe`).

## Choix

- Une largeur fait passer le régiment en Ligne (glisser = ligne, comme TW).
- La répartition au prorata est une règle du cœur (un seul `Move` de groupe avec la largeur
  totale) ; l'aperçu reçoit la largeur et la profondeur d'arrivée de chaque régiment.

## Prochaine étape

Tests cœur, puis Godot.
