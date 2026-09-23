# WIP — Ordres du chef en bataille (F10b)

Branche : `feat/battle-orders`.

## État
- [x] Données `data/battle_orders/*.json` (5 ordres) + `data/schemas/battle_order.schema.json` + test pytest.
- [x] `data-model` : `entities/battle_order.rs`, `GameData::battle_orders` (dossier facultatif).
- [x] `sim-campaign` : `battle_setup` remplit `BattleSetup::orders` (2 lignes).
- [x] `sim-battle` : `orders.rs` (catalogue, disponibilité, effets), `Command::LeaderOrder`,
      erreurs FR, `SideResult::no_quarter`, `Unit::dismount` factorisé avec le siège.
- [x] Tests Rust de chaque ordre (`tests/orders.rs`, 18 tests).
- [x] IA tactique (`ai.rs`, `plan_orders`).
- [ ] Pont `get_leader_orders(side)`.
- [ ] UI `game/scripts/battle/leader_orders_bar.gd` + accroche dans `battle_scene.gd`.
- [ ] Smoke, capture `docs/img/battle-leader-orders.png`, doc `docs/design/battle-orders.md`.

## Prochaine étape
Pont GDExtension `get_leader_orders`, puis la barre UI.
