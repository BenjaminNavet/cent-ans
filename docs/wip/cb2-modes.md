# CB2 — Modes d'unité, icônes d'état, remappage des touches (état)

Branche : `feat/cb2-modes` (depuis `main` 220a1e9f). Plan :
`docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` (section CB2). Fusion **en dernier** de
la vague 4 (après CB3, CB5, CB6). Cible cargo privée : `CARGO_TARGET_DIR=<worktree>/core/target-cb2`.

## État : cœur en cours

- [x] Données : `data/rules/unit_modes.json` + schéma `unit_modes_rules.schema.json` +
      `tools/tests/test_unit_modes_schema.py`.
- [x] Cœur : `src/modes.rs` (règles, `UnitMode`, `UnitStatus`), `src/sim/modes.rs` (commande
      `set_mode`, recul d'escarmouche, garde, battre en brèche, états), `src/ai_modes.rs` (IA
      défensive) ; champs `Unit.{mode_run, guard, skirmish, melee_mode, breach}` omis du JSON ;
      `Command::SetMode` ; empreinte seulement si un mode s'écarte du défaut.
- [x] Pont : `battle_sim_modes.rs` (champs de `get_units`), RuleValues `unit_modes_*`.
- [ ] Tests cœur `cb2_modes.rs` : 4 échecs à corriger (recul, brèche, rejeu antérieur).
- [ ] Godot : table des raccourcis, remappage, aide F1, icônes de mode, pastilles.
- [ ] Marges EP7/EQ7 avant/après.

## Prochaine étape

Corriger les tests cœur, mesurer les marges (`--test cb2_modes -- --ignored margins`).
