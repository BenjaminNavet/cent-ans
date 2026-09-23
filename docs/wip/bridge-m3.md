# WIP — bridge M3 (cities & economy) GDExtension

Scope: `core/crates/godot-bridge`, `core/checks/`, `docs/design/data-model.md` § 7.4.
Do not touch `game/` or `sim-campaign` (other agents work there).

## State
- `CampaignSim::get_province_city(id)` and `get_faction_economy(id)` implemented
  in `core/crates/godot-bridge/src/campaign_sim.rs`, built by hand with `vdict!`
  (no `json_to_variant` helper needed — kept the existing hand-rolled style).
- `get_faction_summary` extended with `projected_income`, `army_upkeep`,
  `building_upkeep`, `tax_rate`.
- Builds cleanly (`cargo build -p godot-bridge`).

## Next steps
- Extend `core/checks/campaign_sim_check.gd` with the province city / faction
  economy / build order / tax rate scenario described in the task.
- Run `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, `core/build.sh`, Godot import + check.
- Document new methods in `docs/design/data-model.md` § 7.4.
- Delete this file once everything is committed.
