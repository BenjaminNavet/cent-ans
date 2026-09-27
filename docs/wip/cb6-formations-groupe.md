# CB6 — Formations de groupe (état)

Branche : `feat/cb6-group-formations` (depuis `main` cad1ca05). Plan :
`docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` (section CB6 et décisions après relecture).
Relecture historique : `docs/research/cb6-formations.md`.
Cible cargo privée : `CARGO_TARGET_DIR=<worktree>/core/target-cb6`.

## État
- [x] Données : `data/rules/group_formations.json` (6 préréglages) + schéma + `tools/tests/test_group_formations_schema.py`.
- [x] Golden de non-régression : `core/crates/sim-battle/tests/fixtures/cb6_deploy_golden.json` (écrit sur le code d'avant CB6).
- [x] Cœur : `sim-battle/src/group_formation.rs` (`layout`, `BattleSim::formation_slots`), `deploy()` et `ai_deploy` via « Ligne de bataille » (golden identique au bit près).
- [ ] Pont : `formation_presets()`, `formation_slots(...)`.
- [ ] Godot : sélecteur, raccourcis, déploiement, bataille.
- [ ] Tests Godot + script de capture.

## Prochaine étape
Tests cœur par préréglage, puis pont.
