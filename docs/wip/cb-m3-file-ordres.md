# CB-M3 — Ordres en file (état)

Branche : `feat/cb-m3-queue` (depuis `main` 88ec35be). Plan :
`docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` (section CB-M3, écart 6).
Cible cargo privée : `CARGO_TARGET_DIR=<worktree>/core/target-cbm3`.

## État : en cours

- [x] Données : `data/rules/battle_queue.json` (`max_queued_orders` = 8) + schéma
      `battle_queue_rules.schema.json` + `tools/tests/test_battle_queue_schema.py`.
- [x] Cœur : `Command::Move`/`Attack` + `queue` (`serde(default)`, omis du JSON quand faux :
      rejeux antérieurs inchangés) ; `Unit.order_queue` ; `QueuedOrder`, `QueueRules`
      (`src/queue.rs`) ; `sim/queue.rs` (`start_move`, `start_attack`, `check_queue_room`,
      `queue_anchor`, `next_queued`) ; `CommandError::QueueFull` ; dépilage dans
      `resolve_movement` ; vidage (ordre sans file, halte, retraite, muraille, déroute, décision,
      pavois, ralliement, scénario) ; `state_digest` (file hachée seulement si non vide).
- [x] Tests `sim-battle/tests/cb_queue.rs` (12 tests, verts).
- [x] `preview_path_from` + `preview_group_queued` (cœur), pont `battle_sim_queue.rs` (`preview_path_from`, `preview_paths_queued`, `get_units.queue`), RuleValues `battle_queue_max`.
- [ ] Godot : Maj + clic droit, points numérotés, file pleine.
- [ ] Tests Godot + sonde de capture.

## Prochaine étape

Godot : `battle_input.gd` (Maj + clic droit), `battle_path_preview.gd` (file numérotée), infobulle file pleine.
