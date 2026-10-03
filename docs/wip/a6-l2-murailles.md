# A6-L2 — murailles, engins et assaut (constat M3)

État : implémenté, tests ciblés verts, suite complète en cours.
- Engins : `work_per_wall_level` + `scaling_min_wall_level` (data/rules/siege_engines.json) ; coût = work + niveau x valeur dès niveau 3.
- Assaut : `wall_defence_per_level` / `wall_engine_relief` (data/rules/auto_resolve.json), `BattleContext.wall`, prévision et résolution auto.
- Tests : core/crates/sim-campaign/tests/a6_l2_siege_walls.rs.
Prochaine étape : suite complète, calibrage équilibre éventuel (IA assiège moins tôt).
