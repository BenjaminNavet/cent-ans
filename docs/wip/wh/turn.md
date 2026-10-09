# WH turn — état

Branche `wh/turn`. Spec : `docs/wip/wh/tour.md` §3 points 2, 5, 7, 8, 9, 10. ADR 0281. **FAIT.**

- Cœur : `requires`/`requires_reason` (chronicle.rs), missions de faction (missions.rs, 8 en données), `campaign_stats.rs` + `get_campaign_report`.
- Godot : autosave par saison (5 + `auto_battle`), rapport `always|auto|off`, `MissionTracker`, cloche `mission_due`, bilan de fin.
- Tests : `cargo test -p sim-campaign --test campaign_life wh_turn`, `res://tests/wh_turn_test.gd`.
- Restes : offre de mission en dilemme ; annoter d'autres événements ; échecs de base hors lot (starting_fit, eq2_balance, b7b).
