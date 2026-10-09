# WH turn — état

Branche `wh/turn` (worktree `../gp-wh-turn`). Spec : `docs/wip/wh/tour.md` §3 points 2, 5, 7, 8, 9, 10. ADR 0281.

- [x] Cœur : `requires`/`requires_reason` des options (chronicle.rs), `allowed/reason` dans `decision_views` + pont.
- [x] Cœur : missions de faction (`faction`, `after`, `province`, cible `fixed`, `MissionsState.done`), 8 missions en données.
- [x] Cœur : `campaign_stats.rs` + `get_campaign_report` (bilan de fin).
- [x] Godot : autosave par saison / 5 emplacements / `auto_battle`, rapport `always|auto|off`, `MissionTracker`, alerte `mission_due`.
- [ ] Godot : bilan de fin (victory_controller), option grisée (chronicle_window), tests.
- [ ] Tests Rust + headless, ADR 0281, ligne lots.md.
